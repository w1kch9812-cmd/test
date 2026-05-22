use std::collections::BTreeMap;

use chrono::Utc;
use sp9_base_layer_config::{R2PublicBase, Version};
use tracing::{info, instrument, warn};

use super::super::manifest::{GoldArtifact, GoldManifest};
use super::super::tippecanoe::LayerKind;
use super::cdn::{cloudflare_purge, preflight_cdn_config, CdnPurgeOutcome};
use super::staging::read_staging_artifact;
use super::PromoteError;
use crate::r2_upload::R2Uploader;

/// promote 입력.
#[derive(Debug, Clone)]
#[cfg(test)]
pub struct PromoteArgs<'a> {
    /// promote 할 version (newtype — invalid 라벨 컴파일 차단).
    pub version: &'a Version,
    /// 검증할 layer 들. 통상 `LayerKind::all_vec().as_slice()` — `Sp9Layer::ALL` SSOT 자동 반영.
    pub layers: &'a [LayerKind],
    /// `tiles_url_template` 의 R2 public host (newtype — scheme/host 검증).
    pub public_url_base: &'a R2PublicBase,
}

/// promote 결과.
#[derive(Debug, Clone)]
#[cfg(test)]
pub struct PromoteResult {
    /// 새 manifest 의 활성 version.
    pub current_version: String,
    /// publish 한 manifest object key.
    pub manifest_key: String,
    /// Round 4 #5 — typed CDN purge outcome (이전 `Option<bool>`).
    pub cdn_purge: CdnPurgeOutcome,
}

/// promote — staging spec 검증 + flat tile 실재 검증 + previous manifest backup +
/// new manifest publish + CDN purge.
///
/// SSS-grade atomicity steps:
/// 0. **Pre-flight** — production env 에서 CDN config 사전 검증 (manifest 만지기 전 fail-fast)
/// 1. 모든 layer staging spec 검증 + 모음.
/// 2. **모든 layer 의 flat tile 실재 검증** — `gold/<version>/<layer>/` list_objects
///    head check (silent R2 drop / partial PUT 차단).
/// 3. **현재 manifest 백업** — `gold/manifest.json` → `gold/manifest.<prev_ver>.json`
///    (없으면 first-publish, 처음에는 prev=None). 즉시 rollback 가능.
/// 4. new manifest 빌드 (`previous_version=<old.current_version>`) + atomic PUT.
/// 5. CDN purge (optional in dev, mandatory in production — step 0 에서 사전 검증).
///
/// # Errors
///
/// - production env 에서 CDN config 누락 → 즉시 abort (manifest 변경 0).
/// - staging spec 누락 (한 layer 라도 미박제) → degrade gracefully (manifest 변경 0).
/// - flat tile 미존재 → degrade (silent drop 잡음).
/// - manifest publish 실패 → degrade (백업 단계 후 publish 전 실패면 backup 만 있음).
/// - CDN purge 실패는 warn (manifest no-cache header fallback) — 단 production 의
///   missing-config 은 step 0 에서 이미 차단됨.
#[allow(clippy::too_many_lines)]
#[cfg(test)]
#[instrument(skip(uploader, args), fields(version = %args.version))]
pub async fn run(
    uploader: &R2Uploader,
    args: &PromoteArgs<'_>,
) -> Result<PromoteResult, PromoteError> {
    // 0. Pre-flight — Round 5 P0 fix (Codex audit "CDN purge 실패가 publish *후* warn"):
    //    production env 에서 CDN config 누락이면 manifest 만지기 전 즉시 abort.
    //    이전 path: step 4 publish 후 step 5 purge 시점에 검출 → manifest 가 stale
    //    CDN 으로 publish 되는 silent partial. 새 path: 0 단계에서 검증.
    preflight_cdn_config()?;

    // 1. 모든 layer 의 staging spec 검증 + 모음.
    let mut artifacts: BTreeMap<String, GoldArtifact> = BTreeMap::new();
    for &layer in args.layers {
        let artifact = read_staging_artifact(uploader, args.version, layer).await?;
        artifacts.insert(layer.layer_name().to_owned(), artifact);
        info!(layer = %layer.layer_name(), "staging spec verified");
    }

    // 2. flat tile 실재 검증 — `gold/<version>/<layer>/` 안에 *최소 1개* .pbf 존재.
    // SSOT: `gold_layer_prefix(version, layer)` helper — trailing `/` 만 추가.
    for &layer in args.layers {
        let prefix = format!(
            "{}/",
            uploader
                .config()
                .gold_layer_prefix(args.version, layer.layer_name())
        );
        let listed = uploader.list_objects(&prefix).await?;
        let pbf_count = listed
            .iter()
            .filter(|o| {
                std::path::Path::new(&o.key)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pbf"))
            })
            .count();
        if pbf_count == 0 {
            return Err(PromoteError::NoFlatTiles {
                layer: layer.layer_name().to_owned(),
                prefix,
            });
        }
        info!(
            layer = %layer.layer_name(),
            pbf_count,
            "flat tile existence verified"
        );
    }

    // 3. 이전 manifest backup (있으면). first publish 는 *expected miss* — breaker 가
    //    failure 로 카운트하지 않도록 `try_get_object_bytes` 사용.
    let manifest_key = uploader.config().manifest_key();
    let previous_version: Option<Version> =
        if let Some(prev_bytes) = uploader.try_get_object_bytes(&manifest_key).await? {
            let prev: serde_json::Value = serde_json::from_slice(&prev_bytes)?;
            // current_version 은 Version newtype — invalid 라벨이 manifest 에 박혀있으면
            // promote 단계에서 `PromoteError::InvalidPreviousVersion` 으로 거부.
            let prev_ver = prev
                .get("current_version")
                .and_then(|v| v.as_str())
                .map(|raw| {
                    Version::new(raw).map_err(|e| PromoteError::InvalidPreviousVersion {
                        raw: raw.to_owned(),
                        detail: e.to_string(),
                    })
                })
                .transpose()?;
            if let Some(ref pv) = prev_ver {
                let backup_key = uploader.config().manifest_backup_key(pv);
                // raw bytes 그대로 PUT — 직렬화 다시 안 함 (sha256 동일 보장).
                let raw: serde_json::Value = serde_json::from_slice(&prev_bytes)?;
                uploader
                    .put_object_json(&backup_key, &raw, "public, max-age=31536000, immutable")
                    .await?;
                info!(backup_key = %backup_key, "previous manifest backed up (rollback ready)");
            }
            prev_ver
        } else {
            info!("no previous manifest — first publish");
            None
        };

    // 4. new manifest 빌드 + publish.
    // P1.3: tiles_url_template — R2Config::tiles_url_template SSOT.
    // `{layer}` 는 클라이언트가 치환할 리터럴 placeholder — 의도적 formatting arg.
    #[allow(clippy::literal_string_with_formatting_args)]
    let tiles_url_template =
        uploader
            .config()
            .tiles_url_template(args.public_url_base, args.version, "{layer}");

    let manifest = GoldManifest {
        current_version: args.version.as_str().to_owned(),
        current_activated_at: Utc::now(),
        previous_version: previous_version.as_ref().map(|v| v.as_str().to_owned()),
        tiles_url_template,
        artifacts,
        manifest_updated_at: Utc::now(),
    };
    uploader
        .put_object_json(&manifest_key, &manifest, "no-cache, max-age=0")
        .await?;
    info!(
        manifest_key = %manifest_key,
        previous_version = ?previous_version,
        "manifest atomically published"
    );

    // 5. CDN purge (Round 4 #5 typed outcome).
    let cdn_purge = match cloudflare_purge(&manifest_key).await {
        Ok(outcome) => outcome,
        Err(e) => {
            warn!(error = %e, "CDN purge failed — manifest no-cache header is fallback");
            CdnPurgeOutcome::Failed
        }
    };

    Ok(PromoteResult {
        current_version: args.version.as_str().to_owned(),
        manifest_key,
        cdn_purge,
    })
}

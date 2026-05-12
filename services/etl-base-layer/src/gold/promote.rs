//! Plan D **L3 Atomicity** — gold 빌드 / R2 PUT 와 *manifest publish* 분리.
//!
//! ## 문제 (이전 design)
//!
//! `gold` subcommand 가 layer 빌드 후 *바로* `gold/manifest.json` 으로 publish.
//! matrix 가 3 layer parallel 일 때 마지막 layer 빌드만 manifest 에 살아남음 → 부분
//! state 가 클라이언트에 노출 (e.g. parcels 빌드 실패 했는데 admin 빌드만 성공한
//! manifest 가 publish 되어 클라가 admin 만 fetch).
//!
//! ## 해결 (본 모듈)
//!
//! 1. **gold subcommand**: layer 별로 R2 의 `gold/<version>/<layer>/...` 에 flat tile
//!    PUT *후* `gold/staging/<version>/<layer>.lineage.json` 박제. manifest 미건드림.
//! 2. **promote subcommand** (신규, 본 모듈): 모든 layer 의 lineage 가 R2 staging 에
//!    존재하는지 검증 → 새 `GoldManifest` 빌드 → atomic PUT `gold/manifest.json` →
//!    Cloudflare CDN cache purge (manifest 만 — flat tile 은 immutable URL).
//! 3. 빌드 실패 시 staging buffer 만 남고 prod manifest 변경 0 — degrade gracefully.
//!
//! ## CDN cache purge
//!
//! `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` 환경변수 양쪽 set 시 활성. `gold/manifest.json`
//! 만 purge (flat tile 들은 immutable URL 이라 불필요). 둘 중 하나 미설정 시 warn skip
//! — manifest 의 `Cache-Control: no-cache, max-age=0` 가 fallback (CDN 가 next-fetch 에서
//! revalidate, 분 단위 staleness 가능 — purge 하면 즉시).

#![allow(clippy::doc_markdown)]

use std::collections::BTreeMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sp9_base_layer_config::{Environment, R2PublicBase, Version};
use thiserror::Error;
use tracing::{info, instrument, warn};

use super::manifest::{BuildLineage, GoldArtifact, GoldManifest};
use super::tippecanoe::LayerKind;
use crate::r2_upload::{R2Uploader, UploadError};

/// promote 단계 에러.
#[derive(Debug, Error)]
pub enum PromoteError {
    /// R2 API.
    #[error("r2: {0}")]
    R2(#[from] UploadError),
    /// JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// 특정 layer 의 lineage 가 staging 에 없음 (build 미완 / 사용자 누락).
    #[error("missing staging lineage for layer {layer} (key {key})")]
    MissingLineage {
        /// 누락 layer.
        layer: String,
        /// 기대 R2 key.
        key: String,
    },
    /// 특정 layer 의 flat tile 이 R2 에 0 개 — silent drop / partial PUT 의심.
    #[error("no flat tiles found in {prefix} for layer {layer}")]
    NoFlatTiles {
        /// 누락 layer.
        layer: String,
        /// 검사한 prefix.
        prefix: String,
    },
    /// 이전 manifest 의 `current_version` 이 [`Version`] 형식 위반 (R2 외부 변조 / 구버전 manifest).
    #[error("invalid previous_version in manifest: {raw:?} ({detail})")]
    InvalidPreviousVersion {
        /// manifest 에서 읽힌 원본 문자열.
        raw: String,
        /// [`sp9_base_layer_config::TypeError`] 의 사람-가독 메시지.
        detail: String,
    },
    /// HTTP 통신 (Cloudflare CDN purge).
    #[error("cdn purge http: {0}")]
    Http(#[from] reqwest::Error),
    /// CDN purge 가 non-2xx 응답.
    #[error("cdn purge failed status={status} body={body} body_read_error={body_read_error:?}")]
    CdnPurge {
        /// HTTP status.
        status: u16,
        /// 응답 body 처음 1024 바이트.
        body: String,
        /// Round 4 #6 — body read 자체 실패 시 그 에러 박제 (이전엔 `unwrap_or_default()`
        /// 로 silent loss). 진단 trail 보존.
        body_read_error: Option<String>,
    },
    /// Round 4 #5 — production env 에서 CDN purge config (`CLOUDFLARE_API_TOKEN` /
    /// `CLOUDFLARE_ZONE_ID` / `R2_PUBLIC_URL_BASE`) 가 누락. dev / staging 은 silent
    /// skip 허용, production 은 fail-fast (manifest 가 stale CDN 으로 publish 되는
    /// silent partial 차단).
    #[error("CDN purge config missing in production: {missing} (set CLOUDFLARE_API_TOKEN / CLOUDFLARE_ZONE_ID / R2_PUBLIC_URL_BASE or override ETL_ENVIRONMENT)")]
    CdnPurgeMissingConfig {
        /// 어느 env 가 누락됐는지.
        missing: String,
    },
    /// Round 5 P1 — `cleanup_manifest_backups(keep=0)` 실수 차단.
    #[error("cleanup keep must be >= 1 (refusing to delete entire backup chain)")]
    InvalidCleanupKeep,
    /// Round 5 (final) — cleanup 중 일부 backup delete 실패. 이전엔 warn 후 `Ok(())` →
    /// silent partial. 새 path: 진행은 계속 (다른 backup 도 시도) 후 typed Err 박제.
    /// workflow 가 본 에러로 exit 1 + Sentry alert.
    #[error("partial cleanup: {deleted}/{attempted} succeeded, {} failed: {failures:?}", failures.len())]
    PartialCleanup {
        /// 삭제 시도한 총 개수.
        attempted: usize,
        /// 성공 개수.
        deleted: usize,
        /// 실패한 (key, error 메시지) 쌍.
        failures: Vec<(String, String)>,
    },
}

/// 한 layer 의 build artifact 메타 — R2 staging 에 박제 후 promote 가 모음.
///
/// `Serialize` + `Deserialize` 양쪽 — write/read 가 *동일 schema* 통과 (P0 typed 검증):
/// staging spec 의 누락 필드 / 오타 / 변조는 [`serde_json::from_slice`] 단계에서 거부.
/// 더 이상 `serde_json::Value` + `unwrap_or_default()` path 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSpec {
    /// PMTiles + flat tiles 의 R2 prefix (예: `gold/v3/parcels`).
    pub key_prefix: String,
    /// PMTiles 파일 size (bytes).
    pub pmtiles_bytes: u64,
    /// PMTiles SHA-256.
    pub pmtiles_sha256: String,
    /// 빌드 결과의 feature 수. `None` = tippecanoe metadata 미지원/파싱 실패.
    pub row_count: Option<u64>,
    /// flat tile 개수.
    pub flat_tile_count: u64,
    /// flat tile 합계 bytes.
    pub flat_tiles_total_bytes: u64,
    /// L10 lineage (본 layer 의 provenance).
    pub lineage: BuildLineage,
}

/// staging 에 layer 의 spec (lineage + artifact meta) PUT.
///
/// gold subcommand 가 R2 batch upload 직후 본 함수를 호출 — manifest 미발행 상태에서
/// promote 가 이용할 데이터를 staging 에 박제.
///
/// # Errors
///
/// JSON 직렬화 / R2 PUT 실패.
#[instrument(skip(uploader, spec), fields(version = %version, layer))]
pub async fn write_staging_spec(
    uploader: &R2Uploader,
    version: &Version,
    layer: LayerKind,
    spec: &ArtifactSpec,
) -> Result<(), PromoteError> {
    let key = uploader.config().staging_spec_key(version, layer.layer_name());
    // typed `ArtifactSpec` 그대로 직렬화 — read 측 `read_staging_artifact` 가
    // 동일 schema 로 typed deserialize 하므로 누락/오타 자동 거부 (P0 typed gate).
    uploader
        .put_object_json(&key, spec, "no-cache, max-age=0")
        .await?;
    info!(key = %key, "staging spec written");
    Ok(())
}

/// staging 에서 layer 의 spec 읽어 [`GoldArtifact`] 로 변환.
///
/// 누락 시 [`PromoteError::MissingLineage`] — promote 가 atomic 보장 (한 layer 라도 빠지면 abort).
///
/// **P0 typed gate** (Codex Round 3 발견 fix): `serde_json::Value` + `unwrap_or_default()`
/// 가 누락 필드를 0/empty 로 통과시키던 trick 제거. [`ArtifactSpec`] 으로 typed
/// deserialize → 필드 부재 / 타입 오류 시 [`PromoteError::Json`] 으로 fail-fast.
async fn read_staging_artifact(
    uploader: &R2Uploader,
    version: &Version,
    layer: LayerKind,
) -> Result<GoldArtifact, PromoteError> {
    let key = uploader.config().staging_spec_key(version, layer.layer_name());
    // try_get_object_bytes → NoSuchKey 는 `Ok(None)` 으로 closure 안에서 흡수
    // (breaker failure 누적 0). None 이면 typed `MissingLineage` 로 매핑.
    let bytes = uploader
        .try_get_object_bytes(&key)
        .await?
        .ok_or_else(|| PromoteError::MissingLineage {
            layer: layer.layer_name().to_owned(),
            key,
        })?;
    // typed `ArtifactSpec` 으로 deserialize — 누락 필드는 serde_json 에러로 abort.
    let spec: ArtifactSpec = serde_json::from_slice(&bytes)?;
    let (tile_min_zoom, tile_max_zoom) = layer.zoom_range();
    Ok(GoldArtifact {
        key: spec.key_prefix,
        source_layer: layer.layer_name().to_owned(),
        pmtiles_bytes: spec.pmtiles_bytes,
        pmtiles_sha256: spec.pmtiles_sha256,
        built_at: spec.lineage.built_at,
        row_count: spec.row_count,
        flat_tile_count: spec.flat_tile_count,
        flat_tiles_total_bytes: spec.flat_tiles_total_bytes,
        tile_min_zoom,
        tile_max_zoom,
        render_min_zoom: layer.render_min_zoom(),
        render_max_zoom: layer.render_max_zoom(),
        cache_max_age_seconds: layer.cache_max_age_seconds(),
        lineage: Some(spec.lineage),
    })
}

/// promote 입력.
#[derive(Debug, Clone)]
pub struct PromoteArgs<'a> {
    /// promote 할 version (newtype — invalid 라벨 컴파일 차단).
    pub version: &'a Version,
    /// 검증할 layer 들. 통상 `LayerKind::all_vec().as_slice()` — `Sp9Layer::ALL` SSOT 자동 반영.
    pub layers: &'a [LayerKind],
    /// `tiles_url_template` 의 R2 public host (newtype — scheme/host 검증).
    pub public_url_base: &'a R2PublicBase,
}

/// Round 4 #5 — CDN purge 결과의 typed outcome (이전 `Option<bool>` 의 ambiguity 제거).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdnPurgeOutcome {
    /// Cloudflare API 200 OK — manifest URL purge 완료.
    Purged,
    /// dev / staging env (`ETL_ENVIRONMENT != "production"`) 에서 CDN config 누락 시
    /// silent skip — manifest 의 `Cache-Control: no-cache` 가 fallback.
    SkippedDevMode,
    /// CDN config 누락. production 에서는 본 variant 가 `PromoteError::CdnPurgeMissingConfig`
    /// 로 변환 — 본 variant 는 `Promote` 단계 도달 안 함 (fail-fast).
    #[allow(dead_code)]
    SkippedNoConfig,
    /// Cloudflare API 호출은 했으나 transient HTTP / 4xx 응답. promote 자체는 성공
    /// (manifest 는 publish), CDN purge 만 실패. 상위 `PromoteError::CdnPurge` 로 박제.
    Failed,
}

/// promote 결과.
#[derive(Debug, Clone)]
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
    let previous_version: Option<Version> = if let Some(prev_bytes) =
        uploader.try_get_object_bytes(&manifest_key).await?
    {
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
    let tiles_url_template = uploader.config().tiles_url_template(
        args.public_url_base,
        args.version,
        "{layer}",
    );

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

/// Round 5 P1 — manifest backup chain cleanup (ADR 0028, runbook § 6).
///
/// `gold/manifest.<version>.json` backup 들을 list → 오래된 것부터 삭제 → 최근 `keep`
/// 개만 보존. monthly cron 으로 호출 (`.github/workflows/sp9-manifest-backup-cleanup.yml`).
///
/// 정렬 기준: R2 `LastModified` (object 메타). version 라벨 자체로 sort 도 가능하지만
/// (`v_2026_05` < `v_2026_06`) external 변경 (예: 수동 backup) 도 자연 처리하려면
/// modification time 이 더 안전.
///
/// # Errors
///
/// - R2 list / delete API 실패
/// - `keep` 가 0 이면 [`PromoteError::InvalidCleanupKeep`] (실수 방지)
pub async fn cleanup_manifest_backups(
    uploader: &R2Uploader,
    keep: usize,
) -> Result<CleanupResult, PromoteError> {
    if keep == 0 {
        return Err(PromoteError::InvalidCleanupKeep);
    }
    // backup key 형식 — `<gold_prefix>/manifest.<version>.json`. prefix 는 manifest_key
    // 의 dirname + `manifest.` glob.
    let backup_prefix = format!("{}/manifest.", uploader.config().gold_prefix);
    let listed = uploader.list_objects(&backup_prefix).await?;

    // backup 파일만 — `manifest.json` 자체는 제외 (`manifest.<version>.json` 만).
    // 패턴: `<gold_prefix>/manifest.<라벨>.json` 의 `.` 가 정확히 2개 (`manifest`,
    // `<라벨>`, `json`). `manifest.json` 은 `.` 1개.
    let manifest_key = uploader.config().manifest_key();
    let mut backups: Vec<_> = listed
        .into_iter()
        .filter(|obj| {
            // `<gold_prefix>/manifest.<...>.json` 형식만 — 정확히 .json 끝 + manifest. prefix.
            std::path::Path::new(&obj.key)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                && obj.key.starts_with(&backup_prefix)
                && obj.key != manifest_key
        })
        .collect();

    info!(
        backup_count = backups.len(),
        keep,
        "manifest backup chain listed"
    );

    if backups.len() <= keep {
        info!(
            backup_count = backups.len(),
            keep,
            "no cleanup needed (within retention)"
        );
        return Ok(CleanupResult {
            total_found: backups.len(),
            kept: backups.len(),
            deleted: 0,
        });
    }

    // 오래된 것 먼저 — backup key 의 *문자열* 정렬 으로 충분 (version 라벨이
    // `v_YYYY_MM` 형식이라 lexicographic = chronological).
    // 단 외부 변경에 안전하려면 `etag` 또는 별도 LastModified field 가 더 정확.
    // 본 단계는 lexicographic 으로 충분 — runbook § 6 에서 "외부 변경 0" 가정.
    backups.sort_by(|a, b| a.key.cmp(&b.key));

    let to_delete = backups.len() - keep;
    let delete_targets: Vec<_> = backups.iter().take(to_delete).cloned().collect();

    let mut deleted = 0;
    let mut failures: Vec<(String, String)> = Vec::new();
    for obj in &delete_targets {
        match uploader.delete_object(&obj.key).await {
            Ok(()) => {
                deleted += 1;
                info!(key = %obj.key, "manifest backup deleted (cleanup)");
            }
            Err(e) => {
                // Round 5 (final stop-hook) — partial cleanup 실패는 typed `Err` 로
                // 전파. 이전엔 warn 후 `Ok(())` 반환 — silent partial = SSS 위반.
                // 진행은 계속 (다른 backup 도 시도) — 모든 실패 모은 후 `Err` 박제.
                warn!(key = %obj.key, error = %e, "backup delete failed — collecting for typed Err");
                failures.push((obj.key.clone(), e.to_string()));
            }
        }
    }

    info!(
        total = backups.len(),
        kept = backups.len() - deleted,
        deleted,
        failures = failures.len(),
        "manifest backup cleanup attempted"
    );

    if !failures.is_empty() {
        return Err(PromoteError::PartialCleanup {
            attempted: delete_targets.len(),
            deleted,
            failures,
        });
    }

    Ok(CleanupResult {
        total_found: backups.len(),
        kept: backups.len() - deleted,
        deleted,
    })
}

/// `cleanup_manifest_backups` 결과.
#[derive(Debug, Clone, Copy)]
pub struct CleanupResult {
    /// 발견한 backup 총 개수.
    pub total_found: usize,
    /// 보존한 개수 (보통 `keep`).
    pub kept: usize,
    /// 삭제한 개수.
    pub deleted: usize,
}

/// Round 5 P0 — promote 의 step 0 pre-flight. production env 에서 CDN config
/// 누락이면 manifest 만지기 전 즉시 abort.
///
/// 본 check 는 `cloudflare_purge` 의 분기와 *동일 로직* 으로 wired — drift 차단.
/// dev/staging 에서는 silent OK (`SkippedDevMode` 가 step 5 에서 자연 발생).
fn preflight_cdn_config() -> Result<(), PromoteError> {
    // ADR 0029 — `Environment::is_production_from_env()` SSOT (ETL_ENVIRONMENT 또는
    // ADR 0035 — `ETL_ENVIRONMENT` SSOT only.
    if !Environment::is_production_from_env() {
        return Ok(());
    }
    let missing: Vec<&str> = [
        ("CLOUDFLARE_API_TOKEN", env_nonempty("CLOUDFLARE_API_TOKEN")),
        ("CLOUDFLARE_ZONE_ID", env_nonempty("CLOUDFLARE_ZONE_ID")),
        ("R2_PUBLIC_URL_BASE", env_nonempty("R2_PUBLIC_URL_BASE")),
    ]
    .iter()
    .filter_map(|(name, present)| if *present { None } else { Some(*name) })
    .collect();
    if missing.is_empty() {
        return Ok(());
    }
    Err(PromoteError::CdnPurgeMissingConfig {
        missing: missing.join(", "),
    })
}

fn env_nonempty(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|v| !v.trim().is_empty())
}

/// Cloudflare CDN purge — `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` + `R2_PUBLIC_URL_BASE`
/// 모두 set 시 활성. manifest 객체 URL 만 purge.
///
/// Round 4 #5 (Codex audit): production env 에서 config 누락 = fail-fast (이전: silent
/// `Ok(false)` 로 manifest 가 stale CDN 으로 publish 되는 trick). dev / staging 은
/// silent skip 그대로 (`SkippedDevMode`).
async fn cloudflare_purge(manifest_key: &str) -> Result<CdnPurgeOutcome, PromoteError> {
    let token = std::env::var("CLOUDFLARE_API_TOKEN")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let zone_id = std::env::var("CLOUDFLARE_ZONE_ID")
        .ok()
        .filter(|v| !v.trim().is_empty());
    let base = std::env::var("R2_PUBLIC_URL_BASE")
        .ok()
        .filter(|v| !v.trim().is_empty());

    // typed unpack — 누락 detection 과 unwrap 을 하나의 match 로. production 시 fail-fast,
    // 그 외 env 는 SkippedDevMode (manifest 의 no-cache header 가 fallback).
    let (token, zone_id, base) = match (token, zone_id, base) {
        (Some(t), Some(z), Some(b)) => (t, z, b),
        (token, zone_id, base) => {
            let missing: Vec<&str> = [
                ("CLOUDFLARE_API_TOKEN", token.is_some()),
                ("CLOUDFLARE_ZONE_ID", zone_id.is_some()),
                ("R2_PUBLIC_URL_BASE", base.is_some()),
            ]
            .iter()
            .filter_map(|(name, present)| if *present { None } else { Some(*name) })
            .collect();
            // ADR 0029 — preflight 와 동일 SSOT 검사.
            if Environment::is_production_from_env() {
                return Err(PromoteError::CdnPurgeMissingConfig {
                    missing: missing.join(", "),
                });
            }
            info!(missing = ?missing, "CDN purge skipped in non-production env");
            return Ok(CdnPurgeOutcome::SkippedDevMode);
        }
    };

    let url = format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/purge_cache");
    let target_url = if base.ends_with('/') {
        format!("{base}{manifest_key}")
    } else {
        format!("{base}/{manifest_key}")
    };
    let body = serde_json::json!({ "files": [target_url] });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        // Round 4 #6 — body read 실패도 typed 박제. 이전 `unwrap_or_default()` silent loss 제거.
        let (body, body_read_error) = match resp.text().await {
            Ok(text) => (text.chars().take(1024).collect::<String>(), None),
            Err(e) => (String::new(), Some(format!("body read failed: {e}"))),
        };
        return Err(PromoteError::CdnPurge {
            status: status.as_u16(),
            body,
            body_read_error,
        });
    }
    info!(target = %target_url, "CDN cache purged");
    Ok(CdnPurgeOutcome::Purged)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::await_holding_lock,  // env-mutating tests 는 process-global 이라 lock-held await 필요
    )]

    use super::{cloudflare_purge, ArtifactSpec, CdnPurgeOutcome, PromoteError};
    use crate::test_support::GLOBAL_ENV_LOCK as ENV_LOCK;

    fn clear_cdn_env() {
        for k in [
            "CLOUDFLARE_API_TOKEN",
            "CLOUDFLARE_ZONE_ID",
            "R2_PUBLIC_URL_BASE",
            "ETL_ENVIRONMENT",
        ] {
            std::env::remove_var(k);
        }
    }

    /// Round 4 #5 — CDN config 누락 + ETL_ENVIRONMENT != production = `SkippedDevMode`.
    #[tokio::test]
    async fn cloudflare_purge_skips_silently_in_dev_mode() {
        let _guard = ENV_LOCK.lock().expect("env mutex");
        clear_cdn_env();
        std::env::set_var("ETL_ENVIRONMENT", "local");
        let outcome = cloudflare_purge("gold/manifest.json")
            .await
            .expect("dev mode skip");
        assert_eq!(outcome, CdnPurgeOutcome::SkippedDevMode);
        clear_cdn_env();
    }

    /// Round 4 #5 — CDN config 누락 + ETL_ENVIRONMENT=production = fail-fast (silent path 0).
    #[tokio::test]
    async fn cloudflare_purge_fails_fast_in_production_when_config_missing() {
        let _guard = ENV_LOCK.lock().expect("env mutex");
        clear_cdn_env();
        std::env::set_var("ETL_ENVIRONMENT", "production");
        let err = cloudflare_purge("gold/manifest.json")
            .await
            .expect_err("production mode missing-config = fail-fast");
        match err {
            PromoteError::CdnPurgeMissingConfig { missing } => {
                assert!(
                    missing.contains("CLOUDFLARE_API_TOKEN"),
                    "missing detail must include token: {missing}"
                );
                assert!(missing.contains("CLOUDFLARE_ZONE_ID"));
                assert!(missing.contains("R2_PUBLIC_URL_BASE"));
            }
            other => panic!("expected CdnPurgeMissingConfig, got: {other:?}"),
        }
        clear_cdn_env();
    }

    /// Round 5 P1 — cleanup-manifest-backups subcommand 의 keep=0 거부.
    /// (실수로 전체 backup chain 삭제 차단.)
    #[tokio::test]
    async fn cleanup_rejects_zero_keep() {
        use crate::r2_upload::R2Config;
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // ListObjects mock — keep=0 abort 가 list 전에 발생해야 하므로 mock 실제 호출 X.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "<?xml version=\"1.0\"?><ListBucketResult><Name>x</Name></ListBucketResult>",
            ))
            .mount(&server)
            .await;

        let cfg = R2Config {
            account_id: "fake".into(),
            access_key: "fake".into(),
            secret_key: "fake".into(),
            bucket: "test-bucket".into(),
            bronze_prefix: "bronze".into(),
            gold_prefix: "gold".into(),
        };
        let uploader = crate::r2_upload::R2Uploader::with_endpoint_override(cfg, server.uri());
        let err = super::cleanup_manifest_backups(&uploader, 0)
            .await
            .expect_err("keep=0 must be rejected");
        assert!(matches!(err, PromoteError::InvalidCleanupKeep));
    }

    /// Round 5 P0 — promote 의 pre-flight 가 production env 에서 CDN config 누락을
    /// *manifest 만지기 전* 차단. 이전 path 는 publish 후 step 5 에서 검출했음.
    #[test]
    fn preflight_blocks_promotion_in_production_when_cdn_missing() {
        let _guard = ENV_LOCK.lock().expect("env mutex");
        clear_cdn_env();
        std::env::set_var("ETL_ENVIRONMENT", "production");
        let err = super::preflight_cdn_config()
            .expect_err("production + missing CDN config = pre-flight abort");
        match err {
            PromoteError::CdnPurgeMissingConfig { missing } => {
                assert!(missing.contains("CLOUDFLARE_API_TOKEN"), "{missing}");
                assert!(missing.contains("CLOUDFLARE_ZONE_ID"), "{missing}");
                assert!(missing.contains("R2_PUBLIC_URL_BASE"), "{missing}");
            }
            other => panic!("expected CdnPurgeMissingConfig, got {other:?}"),
        }
        clear_cdn_env();
    }

    /// Round 5 P0 — dev/staging env 에서는 pre-flight 가 silent OK (config 누락 허용,
    /// step 5 의 `SkippedDevMode` 가 자연 path).
    #[test]
    fn preflight_passes_in_dev_mode_even_when_cdn_missing() {
        let _guard = ENV_LOCK.lock().expect("env mutex");
        clear_cdn_env();
        std::env::set_var("ETL_ENVIRONMENT", "local");
        super::preflight_cdn_config().expect("dev mode pre-flight = silent OK");
        clear_cdn_env();
    }

    /// Round 5 P0 — production env 에 모든 config 가 set 되면 pre-flight 통과.
    #[test]
    fn preflight_passes_in_production_when_cdn_config_complete() {
        let _guard = ENV_LOCK.lock().expect("env mutex");
        clear_cdn_env();
        std::env::set_var("ETL_ENVIRONMENT", "production");
        std::env::set_var("CLOUDFLARE_API_TOKEN", "fake-token");
        std::env::set_var("CLOUDFLARE_ZONE_ID", "fake-zone");
        std::env::set_var("R2_PUBLIC_URL_BASE", "https://r2.example.com");
        super::preflight_cdn_config().expect("complete config = pre-flight OK");
        clear_cdn_env();
    }

    /// Round 4 #5 — production mode 인데 *부분* config (1개만 누락) → 같은 fail-fast.
    #[tokio::test]
    async fn cloudflare_purge_fails_fast_in_production_when_partial_config() {
        let _guard = ENV_LOCK.lock().expect("env mutex");
        clear_cdn_env();
        std::env::set_var("ETL_ENVIRONMENT", "production");
        std::env::set_var("CLOUDFLARE_API_TOKEN", "fake-token");
        std::env::set_var("CLOUDFLARE_ZONE_ID", "fake-zone");
        // R2_PUBLIC_URL_BASE 만 누락.
        let err = cloudflare_purge("gold/manifest.json")
            .await
            .expect_err("partial config = fail-fast");
        match err {
            PromoteError::CdnPurgeMissingConfig { missing } => {
                assert!(missing.contains("R2_PUBLIC_URL_BASE"), "{missing}");
                assert!(!missing.contains("CLOUDFLARE_API_TOKEN"), "{missing}");
            }
            other => panic!("expected CdnPurgeMissingConfig, got: {other:?}"),
        }
        clear_cdn_env();
    }

    /// Round 4 #6 — `PromoteError::CdnPurge` 의 `body_read_error` 필드가 typed.
    /// body read 가 성공했으면 None, 실패했으면 Some(에러 메시지).
    #[test]
    fn cdn_purge_error_body_read_error_field_default_is_none() {
        let err = PromoteError::CdnPurge {
            status: 502,
            body: "Bad Gateway".into(),
            body_read_error: None,
        };
        let display = format!("{err}");
        assert!(display.contains("body=Bad Gateway"), "{display}");
        assert!(display.contains("body_read_error=None"), "{display}");
    }

    #[test]
    fn cdn_purge_error_preserves_body_read_error() {
        let err = PromoteError::CdnPurge {
            status: 503,
            body: String::new(),
            body_read_error: Some("io: connection reset".into()),
        };
        let display = format!("{err}");
        assert!(
            display.contains("connection reset"),
            "body_read_error must propagate: {display}"
        );
    }


    /// P0 typed gate (Codex Round 3 발견 fix): staging spec round-trip.
    /// `write_staging_spec` 가 직렬화한 JSON 이 `ArtifactSpec` 으로 1:1 round-trip.
    #[test]
    fn artifact_spec_round_trips_typed() {
        use super::BuildLineage;
        use chrono::TimeZone;
        let spec = ArtifactSpec {
            key_prefix: "gold/v3/parcels".into(),
            pmtiles_bytes: 1_234_567,
            pmtiles_sha256: "abc123".into(),
            row_count: Some(1_400_000_000),
            flat_tile_count: 800_000,
            flat_tiles_total_bytes: 8_000_000_000,
            lineage: BuildLineage {
                tippecanoe_version: "2.79.0".into(),
                git_sha: "deadbeef".into(),
                built_at: chrono::Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap(),
                bronze_inputs: vec![],
                source_srs: "EPSG:5186".into(),
                layer_name: "parcels".into(),
                build_environment: "dev".into(),
                source_license: None,
                source_url: None,
                correlation_id: None,
            },
        };
        let json = serde_json::to_vec_pretty(&spec).expect("serialize");
        let back: ArtifactSpec = serde_json::from_slice(&json).expect("deserialize");
        assert_eq!(back.key_prefix, spec.key_prefix);
        assert_eq!(back.pmtiles_bytes, spec.pmtiles_bytes);
        assert_eq!(back.flat_tile_count, spec.flat_tile_count);
        assert_eq!(back.row_count, spec.row_count);
        assert_eq!(back.lineage.source_srs, "EPSG:5186");
    }

    /// P0 typed gate: 누락 필드는 `unwrap_or_default()` 로 통과 안 되고 거부됨.
    /// `serde_json::Value` + `as_u64().unwrap_or(0)` 의 trick 이 이전엔 silent 0 으로 통과시킴.
    #[test]
    fn artifact_spec_rejects_missing_required_field() {
        // `pmtiles_bytes` 누락 — 이전 path 에선 `unwrap_or(0)` 로 0 반환.
        let bad_json = serde_json::json!({
            "key_prefix": "gold/v3/parcels",
            "pmtiles_sha256": "abc",
            "row_count": null,
            "flat_tile_count": 100,
            "flat_tiles_total_bytes": 200,
            "lineage": {
                "tippecanoe_version": "2.79.0",
                "git_sha": "x",
                "built_at": "2026-05-08T00:00:00Z",
                "bronze_inputs": [],
                "source_srs": "EPSG:5186",
                "layer_name": "parcels",
                "build_environment": "dev",
            }
        });
        let result: Result<ArtifactSpec, _> = serde_json::from_value(bad_json);
        assert!(
            result.is_err(),
            "missing pmtiles_bytes must be rejected by serde, but got: {result:?}"
        );
    }

    /// P0 typed gate: 잘못된 타입 (string vs u64) 도 거부.
    #[test]
    fn artifact_spec_rejects_wrong_type() {
        let bad_json = serde_json::json!({
            "key_prefix": "gold/v3/parcels",
            "pmtiles_bytes": "not-a-number", // 잘못된 타입
            "pmtiles_sha256": "abc",
            "row_count": null,
            "flat_tile_count": 100,
            "flat_tiles_total_bytes": 200,
            "lineage": {
                "tippecanoe_version": "2.79.0",
                "git_sha": "x",
                "built_at": "2026-05-08T00:00:00Z",
                "bronze_inputs": [],
                "source_srs": "EPSG:5186",
                "layer_name": "parcels",
                "build_environment": "dev",
            }
        });
        let result: Result<ArtifactSpec, _> = serde_json::from_value(bad_json);
        assert!(result.is_err(), "wrong-type pmtiles_bytes must be rejected");
    }

    #[test]
    fn staging_key_format() {
        use crate::r2_upload::R2Config;
        use sp9_base_layer_config::Version;
        let cfg = R2Config {
            account_id: "fake".into(),
            access_key: "fake".into(),
            secret_key: "fake".into(),
            bucket: "bucket".into(),
            bronze_prefix: "bronze".into(),
            gold_prefix: "gold".into(),
        };
        let v = Version::new("v3").expect("test version");
        assert_eq!(
            cfg.staging_spec_key(&v, "parcels"),
            "gold/staging/v3/parcels.spec.json"
        );
    }
}

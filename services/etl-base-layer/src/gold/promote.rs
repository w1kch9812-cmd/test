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
    /// HTTP 통신 (Cloudflare CDN purge).
    #[error("cdn purge http: {0}")]
    Http(#[from] reqwest::Error),
    /// CDN purge 가 non-2xx 응답.
    #[error("cdn purge failed status={status} body={body}")]
    CdnPurge {
        /// HTTP status.
        status: u16,
        /// 응답 body 처음 1024 바이트.
        body: String,
    },
}

/// 한 layer 의 build artifact 메타 — R2 staging 에 박제 후 promote 가 모음.
#[derive(Debug, Clone)]
pub struct ArtifactSpec {
    /// PMTiles + flat tiles 의 R2 prefix (예: `gold/v3/parcels`).
    pub key_prefix: String,
    /// PMTiles 파일 size (bytes).
    pub pmtiles_bytes: u64,
    /// PMTiles SHA-256.
    pub pmtiles_sha256: String,
    /// 빌드 결과의 feature 수 (legacy 0 가능).
    pub row_count: u64,
    /// flat tile 개수.
    pub flat_tile_count: u64,
    /// flat tile 합계 bytes.
    pub flat_tiles_total_bytes: u64,
    /// L10 lineage (본 layer 의 provenance).
    pub lineage: BuildLineage,
}

/// `gold/staging/<version>/<layer>.spec.json` key (lineage + meta 통합 박제).
#[must_use]
pub fn staging_spec_key(gold_prefix: &str, version: &str, layer_name: &str) -> String {
    format!("{gold_prefix}/staging/{version}/{layer_name}.spec.json")
}

/// staging 에 layer 의 spec (lineage + artifact meta) PUT.
///
/// gold subcommand 가 R2 batch upload 직후 본 함수를 호출 — manifest 미발행 상태에서
/// promote 가 이용할 데이터를 staging 에 박제.
///
/// # Errors
///
/// JSON 직렬화 / R2 PUT 실패.
#[instrument(skip(uploader, spec), fields(version, layer))]
pub async fn write_staging_spec(
    uploader: &R2Uploader,
    version: &str,
    layer: LayerKind,
    spec: &ArtifactSpec,
) -> Result<(), PromoteError> {
    let key = staging_spec_key(&uploader.config().gold_prefix, version, layer.layer_name());
    // serde 친화적인 JSON 표현 — `ArtifactSpec` 의 필드 그대로.
    let payload = serde_json::json!({
        "key_prefix": spec.key_prefix,
        "pmtiles_bytes": spec.pmtiles_bytes,
        "pmtiles_sha256": spec.pmtiles_sha256,
        "row_count": spec.row_count,
        "flat_tile_count": spec.flat_tile_count,
        "flat_tiles_total_bytes": spec.flat_tiles_total_bytes,
        "lineage": spec.lineage,
    });
    uploader
        .put_object_json(&key, &payload, "no-cache, max-age=0")
        .await?;
    info!(key = %key, "staging spec written");
    Ok(())
}

/// staging 에서 layer 의 spec 읽어 [`GoldArtifact`] 로 변환.
///
/// 누락 시 [`PromoteError::MissingLineage`] — promote 가 atomic 보장 (한 layer 라도 빠지면 abort).
async fn read_staging_artifact(
    uploader: &R2Uploader,
    version: &str,
    layer: LayerKind,
) -> Result<GoldArtifact, PromoteError> {
    let key = staging_spec_key(&uploader.config().gold_prefix, version, layer.layer_name());
    // body fetch — head 보다 더 명확한 에러.
    let bytes = match uploader.get_object_bytes(&key).await {
        Ok(b) => b,
        Err(UploadError::GetObject { .. }) => {
            return Err(PromoteError::MissingLineage {
                layer: layer.layer_name().to_owned(),
                key,
            });
        }
        Err(other) => return Err(PromoteError::R2(other)),
    };
    let raw: serde_json::Value = serde_json::from_slice(&bytes)?;

    let lineage: BuildLineage = serde_json::from_value(raw["lineage"].clone())?;
    let (tile_min_zoom, tile_max_zoom) = layer.zoom_range();
    Ok(GoldArtifact {
        key: raw["key_prefix"].as_str().unwrap_or_default().to_owned(),
        source_layer: layer.layer_name().to_owned(),
        pmtiles_bytes: raw["pmtiles_bytes"].as_u64().unwrap_or(0),
        pmtiles_sha256: raw["pmtiles_sha256"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        built_at: lineage.built_at,
        row_count: raw["row_count"].as_u64().unwrap_or(0),
        flat_tile_count: raw["flat_tile_count"].as_u64().unwrap_or(0),
        flat_tiles_total_bytes: raw["flat_tiles_total_bytes"].as_u64().unwrap_or(0),
        tile_min_zoom,
        tile_max_zoom,
        render_min_zoom: layer.render_min_zoom(),
        render_max_zoom: layer.render_max_zoom(),
        cache_max_age_seconds: layer.cache_max_age_seconds(),
        lineage: Some(lineage),
    })
}

/// promote 입력.
#[derive(Debug, Clone)]
pub struct PromoteArgs<'a> {
    /// promote 할 version (예: `v3`).
    pub version: &'a str,
    /// 검증할 layer 들. 통상 `LayerKind::ALL`.
    pub layers: &'a [LayerKind],
    /// `tiles_url_template` 의 R2 public host (예: `https://r2.gongzzang.dev/`).
    pub public_url_base: &'a str,
}

/// promote 결과.
#[derive(Debug, Clone)]
pub struct PromoteResult {
    /// 새 manifest 의 활성 version.
    pub current_version: String,
    /// publish 한 manifest object key.
    pub manifest_key: String,
    /// CDN cache purge 시도 결과 (`Some(true)` = success, `Some(false)` = skipped, `None` = failed).
    pub cdn_purged: Option<bool>,
}

/// promote — 모든 layer staging spec 검증 → manifest publish → CDN purge.
///
/// # Errors
///
/// - lineage 누락 (한 layer 라도 staging 미박제).
/// - manifest publish 실패.
/// - CDN purge 실패는 *warn* 처리 (promote 성공으로 간주, manifest 의 `no-cache` header
///   가 fallback). `CLOUDFLARE_*` 환경변수 미설정 시 `cdn_purged = Some(false)` (skip).
#[instrument(skip(uploader, args), fields(version = args.version))]
pub async fn run(
    uploader: &R2Uploader,
    args: &PromoteArgs<'_>,
) -> Result<PromoteResult, PromoteError> {
    // 1. 모든 layer 의 staging spec 검증 + 모음.
    let mut artifacts: BTreeMap<String, GoldArtifact> = BTreeMap::new();
    for &layer in args.layers {
        let artifact = read_staging_artifact(uploader, args.version, layer).await?;
        artifacts.insert(layer.layer_name().to_owned(), artifact);
        info!(layer = %layer.layer_name(), "staging spec verified");
    }

    // 2. manifest 빌드 + publish.
    let base = if args.public_url_base.ends_with('/') {
        args.public_url_base.to_owned()
    } else {
        format!("{}/", args.public_url_base)
    };
    let mut tiles_url_template = String::with_capacity(128);
    tiles_url_template.push_str(&base);
    tiles_url_template.push_str(&uploader.config().gold_prefix);
    tiles_url_template.push('/');
    tiles_url_template.push_str(args.version);
    #[allow(clippy::literal_string_with_formatting_args)]
    {
        tiles_url_template.push_str("/{layer}/{z}/{x}/{y}.pbf");
    }

    let manifest = GoldManifest {
        current_version: args.version.to_owned(),
        current_activated_at: Utc::now(),
        tiles_url_template,
        artifacts,
        manifest_updated_at: Utc::now(),
    };
    let manifest_key = format!("{}/manifest.json", uploader.config().gold_prefix);
    uploader
        .put_object_json(&manifest_key, &manifest, "no-cache, max-age=0")
        .await?;
    info!(manifest_key = %manifest_key, "manifest atomically published");

    // 3. CDN purge (optional).
    let cdn_purged = match cloudflare_purge(&manifest_key).await {
        Ok(true) => Some(true),
        Ok(false) => Some(false),
        Err(e) => {
            warn!(error = %e, "CDN purge failed — manifest no-cache header is fallback");
            None
        }
    };

    Ok(PromoteResult {
        current_version: args.version.to_owned(),
        manifest_key,
        cdn_purged,
    })
}

/// Cloudflare CDN purge — `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ZONE_ID` + `R2_PUBLIC_URL_BASE`
/// 모두 set 시 활성. manifest 객체 URL 만 purge.
async fn cloudflare_purge(manifest_key: &str) -> Result<bool, PromoteError> {
    let Ok(token) = std::env::var("CLOUDFLARE_API_TOKEN") else {
        return Ok(false);
    };
    let Ok(zone_id) = std::env::var("CLOUDFLARE_ZONE_ID") else {
        return Ok(false);
    };
    let Ok(base) = std::env::var("R2_PUBLIC_URL_BASE") else {
        return Ok(false);
    };
    if token.trim().is_empty() || zone_id.trim().is_empty() || base.trim().is_empty() {
        return Ok(false);
    }

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
        let body_text = resp.text().await.unwrap_or_default();
        let truncated = body_text.chars().take(1024).collect::<String>();
        return Err(PromoteError::CdnPurge {
            status: status.as_u16(),
            body: truncated,
        });
    }
    info!(target = %target_url, "CDN cache purged");
    Ok(true)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn staging_key_format() {
        assert_eq!(
            staging_spec_key("gold", "v3", "parcels"),
            "gold/staging/v3/parcels.spec.json"
        );
    }
}

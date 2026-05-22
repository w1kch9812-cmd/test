use sp9_base_layer_config::Version;
use tracing::{info, instrument};

#[cfg(test)]
use super::super::manifest::GoldArtifact;
use super::super::tippecanoe::LayerKind;
use super::{ArtifactSpec, PromoteError};
use crate::r2_upload::R2Uploader;

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
    let key = uploader
        .config()
        .staging_spec_key(version, layer.layer_name());
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
#[cfg(test)]
pub(super) async fn read_staging_artifact(
    uploader: &R2Uploader,
    version: &Version,
    layer: LayerKind,
) -> Result<GoldArtifact, PromoteError> {
    let key = uploader
        .config()
        .staging_spec_key(version, layer.layer_name());
    // try_get_object_bytes → NoSuchKey 는 `Ok(None)` 으로 closure 안에서 흡수
    // (breaker failure 누적 0). None 이면 typed `MissingLineage` 로 매핑.
    let bytes =
        uploader
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

use tracing::{error, info, warn};

use crate::error::VerifyStepError;
use crate::gold::build::BuildResult;
use crate::gold::spawn::Host;
use crate::gold::tippecanoe::LayerKind;
use crate::gold::verify::{
    self, lonlat_to_tile, TileCoordError, TileExpectation, TileSpec, VerifySpec,
};

fn tile_specs_for_layer(layer: LayerKind) -> Result<Vec<TileSpec>, VerifyStepError> {
    if !matches!(layer, LayerKind::Parcels) {
        return Ok(Vec::new());
    }

    let max_z = layer.zoom_range().1;
    sp9_base_layer_config::VERIFY_LANDMARKS
        .iter()
        .map(|landmark| tile_spec_for_landmark(landmark, max_z))
        .collect()
}

fn tile_spec_for_landmark(
    landmark: &sp9_base_layer_config::VerifyLandmark,
    max_z: u8,
) -> Result<TileSpec, VerifyStepError> {
    let (x, y) = lonlat_to_tile(landmark.lon, landmark.lat, max_z).map_err(|e| {
        log_landmark_tile_error(landmark, &e);
        VerifyStepError::TileCoord(e)
    })?;
    log_landmark_scheduled(landmark, max_z, x, y);
    Ok(TileSpec {
        z: max_z,
        x,
        y,
        expectations: vec![TileExpectation::PropertyEquals {
            key: "pnu".to_owned(),
            value: landmark.pnu.to_owned(),
        }],
    })
}

fn log_landmark_tile_error(
    landmark: &sp9_base_layer_config::VerifyLandmark,
    error: &TileCoordError,
) {
    error!(error = %error, landmark = landmark.label, "invalid landmark tile coordinates");
}

fn log_landmark_scheduled(landmark: &sp9_base_layer_config::VerifyLandmark, z: u8, x: u32, y: u32) {
    info!(
        landmark = landmark.label,
        pnu = landmark.pnu,
        tile = format!("{z}/{x}/{y}"),
        "verify landmark scheduled (JSON property check)",
    );
}

fn verify_disabled() -> bool {
    std::env::var("VERIFY_DISABLE").ok().as_deref() == Some("1")
}

fn verify_min_file_bytes() -> u64 {
    std::env::var("VERIFY_MIN_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(sp9_base_layer_config::NATIONWIDE_PMTILES_MIN_BYTES)
}

pub async fn run_verify(
    host: Host,
    build: &BuildResult,
    layer: LayerKind,
) -> Result<(), VerifyStepError> {
    if verify_disabled() {
        log_verify_disabled();
        return Ok(());
    }

    let min_bytes = verify_min_file_bytes();

    let tile_specs = tile_specs_for_layer(layer)?;
    let spec = VerifySpec {
        pmtiles: &build.output_path,
        layer_name: layer.layer_name(),
        min_file_bytes: min_bytes,
        tile_specs: &tile_specs,
    };
    let result = verify::run(host, &spec).await?;
    info!(
        sha256 = %result.sha256,
        file_bytes = result.file_bytes,
        tiles_passed = result.tiles_passed,
        "L2 verification passed",
    );
    Ok(())
}

fn log_verify_disabled() {
    warn!("VERIFY_DISABLE=1 - verification skipped (dev / micro-fixture only)");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parcels_verification_plan_uses_landmark_tiles_at_max_zoom(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let specs = tile_specs_for_layer(LayerKind::Parcels)?;

        assert_eq!(specs.len(), sp9_base_layer_config::VERIFY_LANDMARKS.len());
        let Some(first) = specs.first() else {
            return Err("expected at least one landmark spec".into());
        };
        assert_eq!(first.z, LayerKind::Parcels.zoom_range().1);
        assert_eq!(first.expectations.len(), 1);
        assert!(matches!(
            &first.expectations[0],
            TileExpectation::PropertyEquals { key, value }
                if key == "pnu" && value == sp9_base_layer_config::VERIFY_LANDMARKS[0].pnu
        ));
        Ok(())
    }
}

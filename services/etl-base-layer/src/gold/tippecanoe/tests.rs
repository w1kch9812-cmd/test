#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use sp9_base_layer_config::Layer as Sp9Layer;

use super::*;
use crate::gold::spawn::Host;

#[test]
fn layer_kind_metadata() {
    assert_eq!(LayerKind::Parcels.layer_name(), "parcels");
    assert_eq!(LayerKind::Parcels.zoom_range(), (14, 17));
    assert_eq!(LayerKind::Admin.layer_name(), "admin");
    assert_eq!(LayerKind::Admin.zoom_range(), (6, 12));
    assert_eq!(LayerKind::Complex.layer_name(), "complex");
    assert_eq!(LayerKind::Complex.zoom_range(), (0, 16));
    assert_eq!(LayerKind::Complex.render_min_zoom(), 0);
}

#[test]
fn all_vec_matches_sp9_layer_all_in_count_and_name() {
    // SSOT 보증 — `LayerKind::all_vec()` 가 `Sp9Layer::ALL` 의 reflection.
    // Sp9Layer 에 새 variant 추가 시 `From<Sp9Layer> for LayerKind` 의 exhaustive
    // match 가 컴파일러 에러로 차단 — 본 test 는 *layer_name 일관성* 추가 검증.
    let kinds = LayerKind::all_vec();
    let layers = Sp9Layer::ALL;
    assert_eq!(
        kinds.len(),
        layers.len(),
        "count drift between LayerKind and Sp9Layer"
    );
    for (kind, layer) in kinds.iter().zip(layers.iter()) {
        assert_eq!(
            kind.layer_name(),
            layer.name(),
            "name drift: LayerKind={kind:?} vs Sp9Layer={layer:?}",
        );
    }
}

/// Round 4 stop-hook fix — `active_vec()` 는 `is_active_in_etl()=true` 만. promote
/// 가 본 함수 통과해야 inactive layer (admin/complex) 의 staging spec 미박제로 인한
/// `MissingLineage` false-positive 차단.
#[test]
fn active_vec_excludes_inactive_layers() {
    let active = LayerKind::active_vec();
    // ADR 0027 — 현재 active = parcels 만.
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].layer_name(), "parcels");
}

/// SSOT alignment — `LayerKind::active_vec` 와 `Sp9Layer::is_active_in_etl` 이
/// 동일한 active set 을 반환해야 함. 둘이 drift 하면 promote / workflow matrix /
/// SSOT JSON 출력 셋이 어긋남.
#[test]
fn active_vec_matches_sp9_is_active_in_etl() {
    let active_names: Vec<&str> = LayerKind::active_vec()
        .iter()
        .map(|k| k.layer_name())
        .collect();
    let expected: Vec<&str> = Sp9Layer::ALL
        .iter()
        .filter(|l| l.is_active_in_etl())
        .map(|l| l.name())
        .collect();
    assert_eq!(active_names, expected);
}

#[tokio::test]
async fn no_inputs_returns_error() {
    let out = PathBuf::from("/tmp/x.pmtiles");
    let args = TippecanoeArgs {
        kind: LayerKind::Parcels,
        inputs: &[],
        output: &out,
    };
    let err = run(Host::Native, &args).await.unwrap_err();
    assert!(matches!(err, TippecanoeError::NoInputs));
}

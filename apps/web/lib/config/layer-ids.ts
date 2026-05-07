/**
 * SP9 ADR 0021 — Vector tile layer ID 트리 (`LAYER_IDS` 패턴, `gongzzang-design-lab`
 * 차용). 본 파일이 *프론트 측 layer ID SSOT*. listing-map / dev-x9-test 등이 본 상수
 * 만 import — string literal 흩어짐 0.
 *
 * **Source ID** = ETL `LayerKind::layer_name()` 와 1:1 매칭 (manifest `artifacts` key).
 * **Layer ID** = mapbox-gl `addLayer({ id })` 식별자. 한 source 에 여러 layer 가능
 * (fill / outline / 3D extrusion 등) — naming convention `<source>-<style>`.
 */

/** ETL `LayerKind::layer_name()` 와 1:1 매칭. manifest `artifacts` key. */
export const SOURCE_IDS = {
  parcels: "parcels",
  admin: "admin",
  complex: "complex",
} as const;

export type SourceId = (typeof SOURCE_IDS)[keyof typeof SOURCE_IDS];

/** mapbox-gl `addLayer({ id })` 식별자. `<source>-<style>` 컨벤션. */
export const LAYER_IDS = {
  parcels: {
    fill: "parcels-fill",
    outline: "parcels-outline",
  },
  admin: {
    fill: "admin-fill",
  },
  complex: {
    fill: "complex-fill",
  },
} as const;

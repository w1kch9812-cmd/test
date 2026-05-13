# ADR 0020 — Naver Maps gl SDK 의 vector 한계 + 우리 platform 의 interaction model

| | |
|---|---|
| 작성일 | 2026-05-07 |
| 상태 | Accepted |
| 선행 | [ADR 0017](./0017-listing-marker-render-canvas-bitmap-stamp.md), [ADR 0018](./0018-pnu-first-identity-no-coordinates.md), [ADR 0019](./0019-pmtiles-source-via-addsourcetype.md) |

## 결정

Naver Maps gl SDK 의 *vector layer* 들은 *시각적 base 용도* 로만 사용. 사용자 *interaction 의 대상 vector* (필지 / 산업단지 / 행정구역 / 건물) 는 *우리가 직접 관리* — 우리 PMTiles 또는 V-World 별도 fetch.

이 결정은 SP9 의 본질적 근거. ADR 0016 + 0018 의 PNU-First identity model 을 *기술적으로* 강화하는 정직 박제.

## 맥락

T3b.x spike 의 일환으로 Naver 의 mapbox-gl 인스턴스 의 vector polygon source / layer 전수조사 (Playwright probe, `var/sample/naver-polygons.json`).

### Probe scope 정정 (2026-05-07 EOD)

본 ADR 의 초기 dump 는 polygon type 만 필터했고, 현재 probe 는 `apps/web/tests/probes/naver-sdk.probe.ts` 의 별도 `probe:naver` 명령으로 관리한다:

```ts
const polygonLayers = layers.filter(
  (l) => l.type === "fill" || l.type === "fill-extrusion",
);
```

따라서 **`symbol` (POI 라벨/아이콘 — 지하철역, 학교, 병원, 주유소, 관공서 등) / `line` (도로 centerline, 행정 경계, 철도) / `circle` (점 POI) / `raster` (3D 건물, 위성) / `heatmap`** 미검증. 본 ADR 의 "Naver vector 의 한계 4개" 는 *polygon 카테고리에 한해서만* 유효.

**후속 probe**: `apps/web/tests/probes/naver-sdk.probe.ts` — polygon-only filter 제거 + multi-viewport (강남/부평/서울역) + `naver.maps.CadastralLayer` 비교 dump. `pnpm --filter @gongzzang/web probe:naver` 로 별도 실행한다.

### 발견 — Naver 의 vector polygon layer 23개 (총 layer 279개 중)

| 카테고리 | source | sourceLayer | 개수 |
|---|---|---|---|
| 도로 polygon | `sample` | `road_polygon` | 9 (도로실폭/일반/주요/고속/대교/...) |
| 도로 시설 polygon | `sample` | `road_facility_polygon` | 6 (지하보도/터널/육교/...) |
| 실내 지도 polygon | `indoorgnd`, `indoor` | `indoor_bg_*_a` | 5 (매장/보도블럭/...) |
| 우리 PMTiles | `parcels`/`admin`/`complex` | (우리 source-layer) | 3 |

### 발견 — Naver vector 의 한계

1. **`feature.id` 없음** — 도로 polygon 의 `queryRenderedFeatures` 결과 모두 `id: undefined`.
   - 즉 `setFeatureState` 호출 불가 → 직접 highlight / selection 불가능.
2. **properties 가 카테고리 메타뿐** — 도로 polygon 16개 모두 동일 `{std_code: "001000020000", cate1: "도로실폭", cate2: "일반도로", ...}`. *개별 도로* 의 unique attribute 없음 (도로명 / 도로 ID / 차선 수 / ...).
3. **건물 footprint vector 노출 X** — Naver basemap 에 building 이 *vector layer 로 없음*. raster 로만 (3D 효과 등) 노출.
4. **필지 cadastral data 없음** — Naver basemap 의 *의도* — 부동산 platform 만 필요한 데이터.

### 즉 — Naver 의 vector 는 *시각 base 전용*

Naver 의 의도된 design — 도로 / 실내 polygon 은 *지도가 보이게 하는 시각 요소*. 사용자가 클릭 / 조작할 대상이 아님. 따라서 stable id / rich properties 가 *없는 게 정상*.

## 우리 platform 의 interaction model

| 사용자 행동 | 처리 | 데이터 소스 |
|---|---|---|
| 매물 마커 클릭 | listing 상세 panel | DB (T5 완료) |
| **필지 polygon 클릭** | PNU 추출 → `/listings?pnu=...` (ADR 0018) | **우리 PMTiles** (parcels) |
| 산업단지 polygon 클릭 | 산단 정보 + 매물 list | **우리 PMTiles** (complex) |
| 건물 polygon 클릭 (FU 40) | building info | **V-World `LT_C_SPBD`** 별도 PMTiles |
| POI 점 클릭 | properties 정보 표시 | Naver `poi4osm` source — `queryRenderedFeatures` |
| 도로 polygon 클릭 | 무시 (의미 없음) | Naver, properties 카테고리만 |
| Naver 건물 raster 클릭 | 무시 (vector 없음) | - |

### setFeatureState 활용 (우리 PMTiles 한정)

PMTilesSource (ADR 0019) 에 `promoteId: "PNU"` 옵션 → vector tile 의 `PNU` attribute 가 mapbox-gl 의 `feature.id` 로 promote → `setFeatureState` 안정 작동.

```ts
// 클릭 시 PNU 의 polygon highlight (우리 PMTiles 만 가능)
mb.setFeatureState(
  { source: "parcels", sourceLayer: "parcels", id: "1168010100107370000" },
  { selected: true }
);
"fill-color": ["case",
  ["boolean", ["feature-state", "selected"], false], "#3B82F6", "#10B981"
]
```

이는 *우리* source 라 가능. Naver source 는 promoteId 못 박음.

## SP9 + ADR 0016/0018 의 정당성 강화

본 ADR 의 진단 결과 = SP9 의 *기술적 본질* :

| ADR | 결정 | 본 ADR 의 강화 |
|---|---|---|
| 0016 (PMTiles 100%) | base layer = R2 정적 PMTiles + V-World ETL | *Naver 가 cadastral data 없으므로* 우리가 채워야 함 — 본질적 불가피 |
| 0018 (PNU-First) | listing identity = PNU, 좌표 검색 X | *Naver 의 vector 는 stable id 없음, 우리 PMTiles 만 PNU stable* — 기술적 근거 |

즉 SP9 의 *우리 PMTiles 정책* 은 over-engineering 아님 — Naver SDK 가 *반드시* 채워줘야 하는 정보를 *주지 않으므로* 우리가 직접 만드는 것.

## 거부된 path

### "Naver building polygon 의 setFeatureState 활용"
- Naver vector 에 building 이 없음, feature.id 도 없음 — 불가능.

### "Naver POI 의 styling 직접 변경"
- Naver style 이 internal — 변경 시 SDK 업그레이드 risk.
- POI properties 만 *읽기* 로 활용 (queryRenderedFeatures).

### "Naver 도로 polygon highlight"
- feature.id 없음 + 의미 없음 (사용자가 도로 클릭할 일 없음).

## 후속 (T3b.x + FU 40)

- **FU 40 — `Building.geom` 정확 footprint** — V-World `LT_C_SPBD` 또는 `AL_D194_*` (건물 dataset) 별도 PMTiles. ADR 0020 의 직접 후속.
  - 사용자 *건물 식별 needs* 명시 (2026-05-07) → SP9 finale 이후 *2순위 sub-project* 로 escalate 검토.
- **POI properties 활용** — Naver `poi4osm` source 의 properties 분석 + 매물 검색 시 인접 POI 표시 (지하철역 / 학교 / 등).
- **Symbol/line layer probe 확장** — `apps/web/tests/probes/naver-sdk.probe.ts` 결과 박제 후 *식별 가능 symbol layer* (지하철역, 학교, 관공서) 의 properties schema + feature.id 보유 여부 확인. 결과에 따라 새 ADR (Naver POI runtime 활용 model).
- **Naver `CadastralLayer` 비교** — Naver SDK 가 *별도 옵션* 으로 cadastral overlay 제공 (raster 추정, 약관상 PNU 비공개 가능성). 우리 PMTiles 와 *기능 + 약관* 비교 후 정당성 재확인.

## 참고

- 전수조사 dump: `var/sample/naver-polygons.json` (gitignored)
- bundle 분석: `var/sample/maps-gl.js`
- mapbox-gl `setFeatureState` + `promoteId` 표준: <https://docs.mapbox.com/mapbox-gl-js/api/map/#map#setfeaturestate>

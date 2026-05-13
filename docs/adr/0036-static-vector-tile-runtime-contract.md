# ADR 0036 - Static vector tile runtime contract

| | |
|---|---|
| 작성일 | 2026-05-12 |
| 상태 | Accepted |
| 선행 | [ADR 0013](./0013-listing-search-naver-maps.md), [ADR 0020](./0020-naver-vector-interaction-model.md), [ADR 0021](./0021-static-vector-tile-decomposition.md), [ADR 0027](./0027-admin-complex-layer-source-deferred.md), [ADR 0035](./0035-legacy-r2-removal-and-atomic-namespace.md) |
| 상속/확정 | `platform-core` [ADR 0004 - Static Vector Tile Runtime Contract](../../../platform-core/docs/adr/0004-static-vector-tile-runtime-contract.md) |

## 결정

Gongzzang 지도 runtime 은 **Naver Maps 를 base map provider 로만 사용**하고, 필지/산업단지/행정구역/건물처럼 사용자가 클릭하거나 하이라이트하는 도메인 vector 는 **`platform-core` Catalog 소유의 정적 vector tile contract** 로 제공받는다.

RDS/PostGIS 기반 실시간 vector tile server 는 비용과 운영 복잡도 때문에 채택하지 않는다. PMTiles 는 build artifact 로만 사용하고, browser runtime 은 ADR 0021 의 flat `.pbf` 정적 tile 을 읽는다.

runtime contract 의 물리 pointer 는 `gold/manifest.json` 이고, 논리 owner 는 `platform-core` Catalog 이다. Gongzzang 은 `/catalog/v1/vector-tiles/manifest` 또는 public R2/CDN manifest URL 을 읽는 **manifest consumer only** 이며, manifest version 이나 artifact metadata 를 write 하지 않는다.

```text
gold/manifest.json
gold/manifest.<previous_version>.json
gold/<version>/<layer>.json
gold/<version>/<layer>/{z}/{x}/{y}.pbf
```

`gold/manifest.json` 은 활성 version, rollback hint, layer artifact metadata, `tiles_url_template` 을 담는 SSOT 이다. flat `.pbf` tile 은 immutable URL 로 배포하고, manifest pointer 전환과 rollback 은 `platform-core` Catalog 가 담당한다.

Naver 내부 tile URL, 예를 들어 `https://map.pstatic.net/.../getTile/.../pbf`, 은 Naver SDK 의 내부 구현 detail 이며 Gongzzang 의 도메인 source 또는 SSOT 로 사용하지 않는다.

## 컨텍스트

현재 프론트는 Naver Maps SDK 를 `apps/web/app/layout.tsx` 에서 동기 로드하고, `gl: true` 로 내부 mapbox-gl backend 를 활성화한다. `apps/web/components/listings/listing-map.tsx` 는 `_mapbox` 에 접근한 뒤 `type: "vector"` source 를 추가한다.

Gongzzang ETL 쪽은 과거 L3 atomicity 기반 manifest publish 를 구현했다. platform-core cutover 이후 해당 write path 는 legacy fallback 으로만 남기고 CLI/workflow 에서 비활성화한다.

- `services/etl-base-layer/src/gold/manifest.rs` - legacy `GoldManifest`, `GoldArtifact`, lineage schema
- `services/etl-base-layer/src/gold/promote.rs` - legacy staging spec 검증과 manifest atomic publish 구현
- `services/etl-base-layer/src/r2_upload.rs` - TileJSON key, manifest key, tiles URL template helper

프론트 runtime 은 더 이상 `NEXT_PUBLIC_TILES_BASE_URL/<layer>.json` 을 직접 참조하지 않는다. `NEXT_PUBLIC_TILES_MANIFEST_URL` 또는 `NEXT_PUBLIC_PLATFORM_CORE_BASE_URL` 로 platform-core manifest 를 읽고, active version pointer 를 manifest 에서만 해석한다.

## 거부한 옵션

### A. RDS/PostGIS 실시간 vector tile server

거부한다.

정적 성격이 강한 전국 필지/산업단지/행정구역 데이터에 요청마다 DB query, MVT encode, cache invalidation 을 붙이면 비용과 장애면이 커진다. 사용자의 비용 제약과 AGENTS.md 의 SSS 기둥 중 안전성/가시성/단일 출처 원칙에 맞지 않는다.

### B. Naver 내부 vector 또는 tile endpoint 를 도메인 source 로 사용

거부한다.

Naver 내부 source 는 stable PNU, feature id, domain property, versioning, rollback, license lineage 를 Gongzzang 이 보장할 수 없다. Naver SDK 가 제공하는 vector 는 시각 base 또는 보조 read-only signal 로만 취급한다.

### C. PMTiles direct browser runtime 재도입

거부한다.

ADR 0019 의 Service Worker / `addSourceType` / Blob URL path 는 spike 에서 유지 비용과 내부 구현 의존이 드러났고, ADR 0021 의 flat `.pbf` path 가 더 표준적이다.

### D. 현재 `NEXT_PUBLIC_TILES_BASE_URL/<layer>.json` direct mode 를 영구 contract 로 고정

거부한다.

direct TileJSON mode 는 dev smoke 와 rollback 이전 단계에는 충분하지만, runtime SSOT 가 build-time environment 에 박히기 쉽다. production contract 는 `gold/manifest.json` 으로 현재 활성 version 을 읽는 방식이어야 한다.

## 채택한 contract

### Manifest ownership

1. `gold/manifest.json` owner 는 `platform-core` Catalog 다.
2. Gongzzang 은 manifest consumer only 다.
3. Gongzzang 의 `promote`, rollback, manifest backup cleanup, R2 prefix lifecycle workflow 는 비활성화한다.
4. manifest pointer flip, rollback, artifact lifecycle, lineage 연결은 platform-core Catalog API/ETL 이 담당한다.

### Manifest schema

Gongzzang runtime 이 의존하는 최소 contract 는 다음 필드다.

```json
{
  "schema_version": 1,
  "current_version": "v2026_05",
  "previous_version": "v2026_04",
  "tiles_url_template": "https://static.example.com/gold/{version}/{layer}/{z}/{x}/{y}.pbf",
  "published_at": "2026-05-12T00:00:00Z",
  "artifacts": {
    "parcels": {
      "source_layer": "parcels",
      "tile_min_zoom": 8,
      "tile_max_zoom": 16,
      "render_min_zoom": 14,
      "render_max_zoom": 22,
      "tilejson_object_key": "gold/v2026_05/parcels.json",
      "object_key_prefix": "gold/v2026_05/parcels/",
      "lineage": {
        "source_record_id": "uuid",
        "manifest_file_asset_id": "uuid",
        "tilejson_file_asset_id": "uuid",
        "source_file_asset_ids": ["uuid"]
      }
    }
  }
}
```

`artifacts[layer]` 의 `source_layer`, `tile_min_zoom`, `tile_max_zoom`, `render_min_zoom`, `render_max_zoom` 는 layer 별 rendering contract 이며 Gongzzang 이 override 하지 않는다. `lineage.source_record_id` 와 `lineage.*file_asset_id` 는 platform-core Catalog 의 `source_record` / `file_asset` 에 연결된다.

### Frontend runtime

production frontend 는 최종적으로 다음 순서를 따른다.

1. `NEXT_PUBLIC_TILES_MANIFEST_URL` 이 있으면 public R2/CDN manifest 를 직접 가져온다.
2. 없으면 `NEXT_PUBLIC_PLATFORM_CORE_BASE_URL/catalog/v1/vector-tiles/manifest` 에서 manifest 를 가져온다.
3. manifest schema 를 parse 하고 `parcels` core artifact 존재를 검증한다.
4. manifest 의 `artifacts` 에 존재하는 layer 만 등록한다.
5. source id 는 manifest artifact key 와 일치시킨다.
6. tile URL 은 `tiles_url_template` 에 `{version}` 과 `{layer}` 를 치환해 만든다.
7. `parcels` 는 core layer 이며 실패 시 structured error 를 남긴다.
8. `admin` / `complex` 는 ADR 0027 에 따라 optional layer 로 취급한다.
9. manifest fetch 실패 또는 `_mapbox` probe 실패 시 polygon layer 만 비활성화하고 Naver base map 과 listing marker 는 유지한다.

### Naver compatibility

Naver SDK 에 대해 의존하는 contract 는 다음으로 제한한다.

- SDK script load 성공
- `gl: true` map 생성 성공
- `_mapbox` handle 접근 가능
- `addSource({ type: "vector" })` 가능
- sample Gongzzang tile load 가능
- `queryRenderedFeatures` 또는 click event 에서 `PNU` property 확인 가능

Naver 내부 tile URL, internal source name, internal style version UUID 는 의존하지 않는다.

## 검증 gate

### ETL gate

- flat tile count > 0
- flat tile total bytes > 0
- configured landmark tile 존재
- PMTiles sha256 박제
- bronze input sha256 박제
- source SRS / target SRS 박제
- source license / source URL 박제
- production CDN purge config 누락 시 manifest 변경 전 fail-fast

### Frontend gate

- manifest schema parse test
- missing optional layer skip test
- missing core `parcels` layer error test
- Naver SDK `_mapbox` compatibility Playwright probe
- sample tile non-404 probe
- PNU click -> `parcel` panel open probe
- canvas nonblank probe
- mobile viewport nonblank probe

### Cost visibility gate

매 build 또는 promote 결과는 layer 별 다음 값을 남긴다.

- flat tile count
- flat tile total bytes
- average tile bytes
- max tile bytes
- object count estimate
- active version
- previous version

초기 smoke evidence 는 [2026-05-12 static tile smoke](../research/2026-05-12-static-vector-tile-smoke.md) 에 기록한다.

## 영향

### 긍정적 영향

- RDS tile server 비용과 runtime query 장애면을 제거한다.
- Naver UX 를 유지하면서 도메인 interaction 은 Gongzzang 소유 tile 로 보장한다.
- manifest pointer flip 으로 rollback 과 추적성이 명확하다.
- flat tile 은 CDN/R2 cache 효율이 높다.

### 부정적 영향

- `_mapbox` private API 의존은 남는다. 이를 제거하려면 Naver Maps 를 base map 으로 쓰는 현재 UX 결정을 재검토해야 한다.
- 정적 tile 은 실시간 반영에 약하다. data freshness 는 ETL cadence 로 관리한다.
- flat tile 은 object count 가 증가한다. cost visibility gate 로 지속 관찰한다.

## 재검토 트리거

- Naver SDK update 로 `_mapbox` probe 가 실패한다.
- R2 object count 또는 egress 비용이 월 예산을 초과한다.
- 필지/산단 데이터 freshness 요구가 ETL cadence 보다 짧아진다.
- MapLibre 전환이 Naver base map UX / license risk 보다 더 낮은 비용으로 평가된다.
- platform-core cutover 이후 Catalog owner 가 tile manifest contract 를 별도 service boundary 로 이동한다.

## 참고

- [ADR 0019](./0019-pmtiles-source-via-addsourcetype.md) - rejected PMTiles runtime paths
- [ADR 0020](./0020-naver-vector-interaction-model.md) - Naver vector limitation
- [ADR 0021](./0021-static-vector-tile-decomposition.md) - flat `.pbf` static hosting
- [ADR 0027](./0027-admin-complex-layer-source-deferred.md) - admin/complex optional layer
- [ADR 0035](./0035-legacy-r2-removal-and-atomic-namespace.md) - strict R2 namespace
- [Naver SDK data source audit](../research/2026-05-11-naver-sdk-data-sources.md)

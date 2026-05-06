# Sub-project 4-iii-e: R2 PMTiles Reader + FU 40 (Building footprint) — Spec

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 선행 | SP4-ii (V-World), SP4-iii-a (data.go.kr 건축물대장), SP2b-ii (R2 Reader port 4) |
| 후속 | SP4-iii-e-2 (Manufacturer / RealTransaction / CourtAuction PMTiles) |
| 추정 | 8-10 task, 2-3일 |

---

## 1. 개요

`crates/domain/core/{parcel,building,industrial-complex,manufacturer}` 와
`crates/domain/market/{real-transaction,court-auction}` 의 **6 Reader trait** 가
포트만 정의된 상태 (SP2b-ii). 실 구현체 = R2 (Cloudflare R2, S3-호환) 위에
정적 PMTiles + JSON 인덱스로 배포된 데이터를 읽는 client.

**우선순위 — SP4-iii-e 1차 (본 SP)**:

1. `crates/data-clients/r2-public-data/` 신규 lib (S3-호환 client + PMTiles 파서)
2. `R2ParcelReader::fetch_markers_in_bbox` — 지도 마커 (FU 30 close)
3. `R2BuildingReader::fetch_by_pnu` — 정확한 footprint (FU 40 close, V-World
   합성 대체)
4. `R2IndustrialComplexReader` (3 메서드)

**SP4-iii-e-2 (별도 후속)**:
- Manufacturer / RealTransaction / CourtAuction PMTiles Reader

이 분리 = 1차 SP 가 *블록 size 적정* (2-3일). 2차는 별도 ETL 파이프라인 + 데이터
정합성 검증이 무거움.

---

## 2. 배경 — R2 + PMTiles 결정 근거

| 옵션 | 적합도 | 비고 |
|---|---|---|
| **R2 정적 PMTiles** ✅ | high | 빌더 1회, reader N대 — beam-quality 부동산 데이터 (parcel/building polygon) 정적 fit. spatial query (bbox, point-in-polygon) PMTiles 내장 |
| Postgres + PostGIS | medium | 1.4억 필지 + 폴리곤 적재 → 100GB+ DB cost. 자주 쓰는 read pattern 은 정적이라 oversized |
| WFS realtime | low | V-World 쿼터 일일 60K — bbox 검색 1 user 가 다 소진 |

→ R2 PMTiles 선택. ETL 파이프라인 (V-World/data.go.kr → PMTiles 빌드 + R2 upload)
은 별도 service `services/etl-pmtiles-builder` (본 SP 미포함, 후속 SP9).
SP4-iii-e 는 *reader* 만.

---

## 3. 범위

### 포함

- 신규 crate `crates/data-clients/r2-public-data/`:
  - `R2Config` + `from_env` (`R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`,
    `R2_SECRET_ACCESS_KEY`, `R2_BUCKET`, `R2_PUBLIC_URL_BASE` 선택)
  - `R2Client` — `aws-sdk-s3` 0.x with R2 endpoint override + circuit breaker
  - `pmtiles_reader::PmtilesReader` — `pmtiles` crate 0.10.x 또는 직접 파싱 (선택 시점 검증). bbox query / point-at(lng, lat) API
  - `parcel::R2ParcelReader` impl `ParcelReader::fetch_markers_in_bbox`
  - `building::R2BuildingReader` impl `BuildingReader::fetch_by_pnu` (footprint)
  - `industrial_complex::R2IndustrialComplexReader` impl 3 메서드
- `Policy::r2_default()` (timeout 8s, retry 1, threshold 5/10s, cooldown 60s
  — 정적 객체라 Government API 보다 관대)
- `RawCapture` 통합 — `source = "r2_public_data"` (audit 동일성)
- 단위 테스트: PMTiles 파서 mock 응답 → marker / footprint 추출
- 통합 테스트: wiremock 으로 S3 endpoint mock + 5+ 시나리오

### 미포함

- **ETL 빌더**: V-World/data.go.kr → PMTiles 변환은 별도 (FU 60)
- **Manufacturer / RealTransaction / CourtAuction Reader**: SP4-iii-e-2
- **R2 캐시 레이어**: object-level cache (Redis) 는 FU 28
- **PMTiles streaming**: 큰 polygon 응답 stream — 1차는 in-memory
- **multi-region R2 failover**: production 단계 인프라

---

## 4. 컴포넌트

### 4.1 `R2Config` + `R2Client`

```rust
pub struct R2Config {
    pub account_id: String,    // <ACCOUNT>.r2.cloudflarestorage.com
    pub access_key_id: String,
    pub secret_access_key: String,
    pub bucket: String,
    pub public_url_base: Option<String>, // 공개 직접 URL (CDN 모드)
}

pub struct R2Client {
    s3: aws_sdk_s3::Client,    // endpoint override = R2
    config: R2Config,
    breaker: Breaker,
    policy: Policy,
}

impl R2Client {
    pub async fn get_object_bytes(&self, key: &str) -> Result<Vec<u8>, BreakerError<...>>;
    pub async fn head_object(&self, key: &str) -> Result<HeadResponse, BreakerError<...>>;
    // pre-signed URL 발급 (SP6-iv photo uploader 가 사용 — FU 56 R2 통합)
    pub fn presigned_put_url(&self, key: &str, expires_in: Duration)
      -> Result<String, ConfigError>;
}
```

R2 endpoint URL pattern:

```text
https://{account_id}.r2.cloudflarestorage.com
```

`aws-sdk-s3` 의 `endpoint_url(...)` override 로 R2 가리킴. AWS 자격증명은 R2
키 (S3-호환).

### 4.2 PMTiles 파서

`pmtiles` crate 사용 — 검증 대상:

- 0.10.x 가 v3 PMTiles spec 호환
- async-friendly?
- bbox-tile 매핑 helper 제공?

대안: Protomaps PMTiles spec 직접 구현. v3 = magic, header(127B), root dir,
leaf dirs, tile data. 현실적으로 reader-only 라 직접 구현 가능 (~500 lines).

본 SP 1차 = 직접 구현 + 단위 테스트로 verify. crate 도입은 검증 후 결정.

### 4.3 `R2ParcelReader::fetch_markers_in_bbox`

```rust
const PMTILES_PARCELS_KEY: &str = "static/parcels.pmtiles";
// scheme: tile (z, x, y) → tile data → MVT (Mapbox Vector Tile) decode
// → features (PNU + center coordinate) → ParcelMarker
```

bbox 가 한 zoom 레벨에 N tiles 매핑. 각 tile fetch (`R2Client.get_object_bytes`,
`pmtiles::tile_at(z, x, y)`) → MVT decode → features filter (bbox 안만).

성능: tile-level cache (LRU `Mutex<HashMap<(z,x,y), Vec<u8>>>`) 권장 — 본 SP
1차 미포함 (FU 28 redis 캐시 와 통합).

### 4.4 `R2BuildingReader::fetch_by_pnu` (FU 40 close)

```rust
const PMTILES_BUILDINGS_KEY: &str = "static/buildings.pmtiles";
const INDEX_PNU_TO_BUILDINGS_KEY: &str = "static/index/pnu_to_buildings.json";
```

`pnu_to_buildings.json` 인덱스 (PNU → building IDs)로 1차 lookup, 그 후
PMTiles 에서 building footprint polygon fetch. 인덱스 file 은 V-World/data.go.kr
ETL 시 함께 빌드 (별도 SP).

**SP4-iii-a 합성 대체 흐름**:

`DataGoKrBuildingReader::fetch_by_pnu` 가 V-World 필지 폴리곤을 합성하던 부분
(`reader.rs::fetch_polygon`) 을 *옵션* 으로 교체:

- `BuildingFootprintSource` enum: `VWorldParcel` (현재) | `R2Pmtiles` (새)
- 호출 측 (composition root) 가 선택. R2 미통합 환경 = `VWorldParcel` fallback
- 본 SP 종료 후 production 은 `R2Pmtiles` 우선 — 정확한 building footprint

이 변경은 SP4-iii-a `building_register::reader` 의 `Arc<VWorldClient>` →
`Arc<dyn BuildingFootprintSource>` 추상화로 일반화. 두 구현체 (vworld /
r2-pmtiles) 모두 trait 구현.

### 4.5 `R2IndustrialComplexReader`

3 메서드 — 산단 도메인. 각 산단 = JSON record + boundary polygon (PMTiles).

```text
static/industrial_complexes/index.json     -- code -> meta
static/industrial_complexes/by_sigungu.json -- sigungu_code -> [code]
static/industrial_complexes/boundaries.pmtiles -- polygon
```

### 4.6 RawCapture 통합

```rust
self.raw_capture.capture(
    /*pnu or code*/ key,
    /*source*/ "r2_public_data",
    &raw_meta_json,
    fetched_at,
).await
```

R2 fetch 응답이 binary (PMTiles tile or polygon) — JSON 형태로는 헤더/메타만
저장. 큰 binary 자체는 R2 에 이미 영구 보존 (캐시 만료 무관).

---

## 5. 검증 기준 (DoD)

1. `R2Config::from_env` + 단위 테스트 4 (5 env 변수, 누락 / 빈 / 정상 / 선택)
2. `R2Client::get_object_bytes` — wiremock 으로 200/404/5xx 시나리오 3
3. PMTiles header 파서 — 단위 테스트 3 (magic / version / metadata)
4. `R2ParcelReader::fetch_markers_in_bbox` — wiremock + sample tile bytes,
   bbox 안의 PNU 만 반환
5. `R2BuildingReader::fetch_by_pnu` — wiremock + sample index JSON +
   sample tile, 정확한 polygon (V-World 합성과 다른 footprint)
6. `R2IndustrialComplexReader::fetch_by_code` / `fetch_by_sigungu` /
   `fetch_in_bbox` — 각 wiremock 시나리오
7. `Policy::r2_default()` 단위 테스트 1
8. workspace.members + clippy `--all-targets -- -D warnings` 그린
9. 5 CI workflow 그린
10. SP4-iii-a `BuildingFootprintSource` 추상화 + R2 구현체 wire-up
    (composition root)

---

## 6. SSS 7기둥

| 기둥 | 적용 |
|---|---|
| 1 일관성 | R2 도 `circuit_breaker::execute` 통과. `RawCapture` source 통일 |
| 2 자동강제 | env-driven config — production 에서 R2 키 미설정 시 명시적 ConfigError (silent fallback X) |
| 3 추적성 | tracing instrument + raw_capture (source=r2_public_data) |
| 4 안전성 | timeout 8s + retry 1 + circuit. 정적 객체라 short timeout 적정 |
| 5 가시성 | tile fetch 빈도 / cache miss 비율 / breaker open 모두 tracing event |
| 6 SSOT | PMTiles 가 R2 단일 source. ETL 빌더 (별도 SP) 가 V-World/data.go.kr 에서 빌드 |
| 7 명확성 | `BuildingFootprintSource` enum/trait 분기 — V-World 합성 (FU 40 까지 fallback) vs R2 정확한 footprint, 명시적 |

---

## 7. Follow-up

- **FU 60**: `services/etl-pmtiles-builder` — V-World/data.go.kr → PMTiles 빌드
- **FU 61**: SP4-iii-e-2 — Manufacturer / RealTransaction / CourtAuction Reader
- **FU 62**: tile-level LRU cache (in-memory)
- **FU 63**: redis 캐시 레이어 (FU 28 와 통합)
- **FU 64**: PMTiles streaming for large response (smart bbox decomposition)
- **FU 65**: multi-region R2 failover
- **FU 66**: ETL freshness monitoring — SP7-iii 가 R2 객체 staleness 검출
- **FU 67**: SP6-iv 의 PhotoUploader 가 R2 presigned URL 사용 — SP4-iii-e
  종료가 unblocker

---

## 8. Risk

- **`pmtiles` crate 가 alpha**: 검증 후 직접 파서로 fallback. 직접 구현 시
  ~500 lines + 풍부한 단위 테스트
- **R2 endpoint override 불안정**: `aws-sdk-s3` 의 endpoint override 가 SigV4
  presigner 와 호환 안 될 수 있음 — 검증 전 PoC 필수
- **MVT decode**: `protozero` 또는 `mvt-rs` 검증
- **데이터 부재**: ETL 빌더 (FU 60) 가 미구현 → 본 SP 의 reader 는 *mock 데이터*
  로 검증. 실제 prod 데이터 적재는 ETL 후
- **SP4-iii-a 변경 범위**: `BuildingFootprintSource` 추상화는 SP4-iii-a 의
  reader 시그니처 일부 변경 — 통합 테스트 갱신 필요

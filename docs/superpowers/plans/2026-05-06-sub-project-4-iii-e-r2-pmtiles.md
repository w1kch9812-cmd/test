# Sub-project 4-iii-e: R2 PMTiles Reader + FU 40 — 계획

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Approved |
| 선행 spec | [`2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md`](../specs/2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md) |
| 추정 | 9 task, 2-3일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp4-iii-e): spec + plan -- R2 PMTiles Reader + FU 40 (Building footprint)`

---

## T2 — `Policy::r2_default()` + 신규 `crates/data-clients/r2-public-data` skeleton

- `crates/circuit-breaker/src/policy.rs` 에 `r2_default()` (timeout 8s,
  retry 1, threshold 5, window 10s, cooldown 60s) + 단위 테스트 2
- `crates/data-clients/r2-public-data/Cargo.toml` 생성:
  - deps: parcel-domain / building-domain / industrial-complex-domain /
    shared-kernel / circuit-breaker / raw-capture-client / aws-sdk-s3 /
    aws-config / async-trait / chrono / geo-types / reqwest / serde /
    serde_json / thiserror / tokio / tracing / bytes
  - dev: wiremock / tokio[full]
- `src/lib.rs` skeleton + `error.rs` (`ConfigError`, `ParseError`)
- workspace.members 추가

**commit**: `feat(sp4-iii-e-t2): Policy::r2_default + r2-public-data crate skeleton`

---

## T3 — `R2Config::from_env` + `R2Client::get_object_bytes`

- `src/client.rs` — `R2Config` (5 env 변수), `R2Client::new` /
  `with_policy` 패턴 (data-go-kr 와 동일)
- `R2Client::get_object_bytes(key)` — `aws-sdk-s3::Client` get_object →
  Bytes. circuit_breaker::execute 통과
- 단위 테스트 4 (config_from_env 4 cases)
- 통합 테스트 3 (wiremock S3 endpoint mock — 200 + body / 404 / 5xx retry)

**commit**: `feat(sp4-iii-e-t3): R2Config + R2Client.get_object_bytes (S3-호환)`

---

## T4 — PMTiles 파서 (직접 구현, ~500 lines)

- `src/pmtiles.rs` — v3 spec 직접 구현:
  - magic byte 검증 (`PMTiles` ASCII)
  - 127-byte header 파싱 (root_offset, root_length, leaf_dirs_offset, ...)
  - directory 디코드 (varint 압축)
  - tile data fetch — `tile_at(z, x, y) -> Option<Bytes>`
- 단위 테스트 5 (header parse / magic mismatch / directory decode /
  tile_at hit / tile_at miss)
- sample 데이터: `crates/data-clients/r2-public-data/tests/fixtures/sample.pmtiles`
  (작은 1-tile fixture, ~10KB)

**commit**: `feat(sp4-iii-e-t4): PMTiles v3 parser (header + directory + tile_at)`

---

## T5 — `R2ParcelReader::fetch_markers_in_bbox` (FU 30 close)

- `src/parcel.rs` — `R2ParcelReader` impl `ParcelReader::fetch_markers_in_bbox`
  - bbox → tile coords (z, x, y) 매핑 (zoom = 14 default)
  - 각 tile fetch via PmtilesReader
  - MVT decode → features → bbox 안만 필터 → `ParcelMarker`
  - `fetch_by_pnu`: SP4-iii-e 1차 미구현 — 별도 endpoint 필요 (PMTiles 는
    spatial; PNU index 는 별도 JSON file)
- 단위 테스트 3 (bbox→tiles 변환, mock tile decode, empty bbox)
- 통합 테스트 2 (wiremock + sample.pmtiles → marker 추출)

**commit**: `feat(sp4-iii-e-t5): R2ParcelReader.fetch_markers_in_bbox (FU 30 closed)`

---

## T6 — `R2BuildingReader::fetch_by_pnu` (FU 40 close)

- `src/building.rs` — `R2BuildingReader` impl `BuildingReader::fetch_by_pnu`
  1. `pnu_to_buildings.json` index fetch + parse (캐시 가능 — 1차 매번 fetch)
  2. PNU → building IDs lookup
  3. PMTiles 에서 각 building footprint polygon fetch
  4. `Vec<Building>` (footprint 정확)
- `fetch_by_id` 미구현 → `Err(Fetch("FU 42"))`
- 단위 테스트 3 (PNU lookup hit/miss/multi)
- 통합 테스트 2 (wiremock + sample fixtures)

**commit**: `feat(sp4-iii-e-t6): R2BuildingReader.fetch_by_pnu (FU 40 closed -- 정확한 footprint)`

---

## T7 — `BuildingFootprintSource` 추상화 + SP4-iii-a 통합

- `crates/data-clients/data-go-kr/src/building_register/footprint.rs` 신규:
  - `BuildingFootprintSource` trait — `async fn fetch_polygon(pnu) -> Polygon`
  - `VWorldFootprintSource` impl (현재 `reader.rs::fetch_polygon` 추출)
  - `R2FootprintSource` impl (R2BuildingReader 첫 building 의 polygon)
- `DataGoKrBuildingReader::new` 가 `Arc<dyn BuildingFootprintSource>` 받게 변경
- 기존 호출자 (composition root) — `services/api/src/main.rs` 또는 spec § 4.4 가
  지정한 구체 (`VWorld...` 가 default, R2 도입 시 swap)
- 통합 테스트 갱신 (wiremock 양쪽 source mock)

**commit**: `refactor(sp4-iii-e-t7): BuildingFootprintSource abstraction (V-World/R2 swap)`

---

## T8 — `R2IndustrialComplexReader` (3 메서드)

- `src/industrial_complex.rs` — 3 메서드 (`fetch_by_code` /
  `fetch_by_sigungu` / `fetch_in_bbox`)
- index.json + by_sigungu.json + boundaries.pmtiles 패턴
- 통합 테스트 3

**commit**: `feat(sp4-iii-e-t8): R2IndustrialComplexReader (3 fetch methods)`

---

## T9 — workspace 검증 + push + SSOT

- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- 로컬 `cargo test --workspace --lib --bins` 그린
- push → 5 CI workflow 그린
- SSOT 갱신:
  - `docs/superpowers/roadmap.md` SP4-iii-e ✅
  - `memory/project_progress.md` SP4-iii-e 본문 + FU 30/40 closed 표기
  - `MEMORY.md` index 갱신

**commit**: `docs(sp4-iii-e-t9): SP4-iii-e 종료 -- R2 PMTiles Reader (Parcel/Building/IC) + FU 30/40 closed`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| circuit-breaker | `policy.rs` | `r2_default()` + 2 unit |
| 신규 crate | `crates/data-clients/r2-public-data/{Cargo.toml, src/{lib,client,error,pmtiles,parcel,building,industrial_complex}.rs}` | 신규 |
| 통합 테스트 | `crates/data-clients/r2-public-data/tests/{r2_client,pmtiles_parser,parcel_reader,building_reader,ic_reader}_integration.rs` | 신규 5 |
| sample fixture | `tests/fixtures/sample.pmtiles` (작은 binary) | 신규 |
| SP4-iii-a refactor | `crates/data-clients/data-go-kr/src/building_register/{footprint.rs,reader.rs}` | 신규 trait + reader 시그니처 |
| workspace | `Cargo.toml` | members 추가 |
| docs | spec + plan + roadmap + project_progress + MEMORY | 신규/갱신 |

총 ~20-25 파일.

---

## 위험 요소

- **`aws-sdk-s3` 0.x volatile**: 0.x 시리즈 breaking change 잦음 — Cargo.lock
  에 정확 버전 pin
- **PMTiles spec v3 호환**: header/directory format 직접 구현이 ETL 빌더 (FU 60)
  의 출력과 맞아야 함. ETL 미존재라 *mock fixture* 가 reader 의 진실 — 1차
  검증은 lab quality. production 적용 전 prod-fixture 회귀 테스트 추가
- **MVT decode 비용**: 단순 PNU + center 만 추출이라 가벼움 — 큰 polygon 응답
  (footprint) 은 별도 분기. 본 SP 에서 protozero crate 검증
- **SP4-iii-a 통합 (T7)**: 시그니처 변경이 `building_register::reader` 의 6 통합
  테스트 영향. 모두 wiremock 양쪽 source 로 갱신 필요
- **로컬 빌드 — `aws-sdk-s3` 가 RT-only deps 다수 (rustls 등)**: 기존 vworld
  와 동일 stack 이라 큰 충돌 없음. 단 빌드 시간 +1 분 정도

# Sub-project 4-iii-a: data.go.kr 건축물대장 + BuildingReader (Spec)

| | |
|---|---|
| 작성일 | 2026-05-04 |
| 상태 | Approved |
| 선행 | SP4-ii (V-World), SP4-iii-d (PgRawCapture), SP2b-ii (`BuildingReader` port) |
| 후속 | SP4-iii-b (실거래가), SP4-iii-c (법제처) |

---

## 1. 개요

V-World 가 *없는* 정보를 채움 — 연면적 / 층수 / 구조 / 높이. 건축물대장 표제부
(`getBrTitleInfo`) API 통합 → `BuildingReader::fetch_by_pnu` 구현체.

V-World 패턴 그대로 답습:
- `crates/data-clients/data-go-kr/` 신규 lib (V-World 와 같은 구조)
- circuit-breaker `Policy::data_go_kr_default()` 추가
- ACL parser (data.go.kr JSON → 도메인 `Building`)
- `PgRawCapture` 첫 실사용 — `source = "data_go_kr_building"`

---

## 2. 범위

### 포함
- `crates/circuit-breaker` 에 `Policy::data_go_kr_default()` 상수 추가:
  - timeout 15s, retry 2회 (1s/2s/4s), threshold 5, window 5s, cooldown 30s
- 신규 crate `crates/data-clients/data-go-kr/`:
  - `DataGoKrConfig` — `service_key`, `base_url` (default `https://apis.data.go.kr`)
  - `DataGoKrClient` — `reqwest::Client` + `Breaker` + `Policy`
  - `building_register::BuildingRegisterClient` — `getBrTitleInfo` 호출
  - `building_register::parser::parse_building_title` — JSON → `Vec<Building>` (한 PNU 다중 건물)
  - `pnu_split` — PNU 19자리 → 분해 파라미터 (`sigunguCd`/`bjdongCd`/`platGbCd`/`bun`/`ji`)
  - `DataGoKrBuildingReader` impl `BuildingReader::fetch_by_pnu` only (`fetch_by_id` 는 후속)
- 통합 테스트 (`crates/data-clients/data-go-kr/tests/`):
  - `building_register_integration.rs`: 6 wiremock 시나리오 (happy / empty / 5xx / malformed / circuit / multi-building)
- 단위 테스트:
  - `pnu_split` 변환 round-trip
  - `parse_building_title` JSON fixture (5-7 케이스)

### 미포함
- `fetch_by_id`: data.go.kr 식별자가 `mgmBldrgstPk` (BigInt-string) — 별도 endpoint, FU
- 토지대장 (`getLandRegInfo` 등) — SP4-iii-a-2 (또는 SP4-iii-b 와 묶음)
- 실거래가 — SP4-iii-b
- 다른 source (data_go_kr_land/realtransaction) — 위 분리

---

## 3. 컴포넌트

### 3.1 `crates/circuit-breaker/src/policy.rs` 추가

```rust
impl Policy {
    pub const fn data_go_kr_default() -> Self {
        Self {
            timeout_ms: 15_000,
            max_retries: 2,
            retry_base_ms: 1_000,
            open_threshold: 5,
            open_window_ms: 5_000,
            open_cooldown_ms: 30_000,
        }
    }
}
```

### 3.2 PNU 분해 (`pnu_split.rs`)

```rust
pub struct PnuParts<'a> {
    pub sigungu_cd: &'a str,    // PNU[0..5]
    pub bjdong_cd: &'a str,     // PNU[5..10]
    pub plat_gb_cd: &'a str,    // PNU[10..11]
    pub bun: &'a str,            // PNU[11..15]
    pub ji: &'a str,             // PNU[15..19]
}

pub fn split(pnu: &Pnu) -> PnuParts<'_>;  // panic-free, infallible (Pnu 가 19자리 invariant)
```

### 3.3 building_register API 호출

```
GET {base_url}/1613000/BldRgstService_v2/getBrTitleInfo?
    ServiceKey={key}
    &sigunguCd={5}
    &bjdongCd={5}
    &platGbCd={1}
    &bun={4}
    &ji={4}
    &numOfRows=100
    &pageNo=1
    &_type=json
```

응답 (data.go.kr 표준):
```json
{
  "response": {
    "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
    "body": {
      "items": {
        "item": [
          {
            "bldNm": "○○동",                    // building_name
            "mainPurpsCdNm": "공장",             // → main_purpose_code
            "strctCdNm": "철골구조",             // → structure_code
            "totArea": "1500.50",                // 문자열! → AreaM2 parse
            "grndFlrCnt": "3",                   // → ground_floors
            "ugrndFlrCnt": "0",                  // → underground_floors
            "heit": "12.0",                      // height_m (선택)
            "useAprDay": "20100315",             // YYYYMMDD → NaiveDate
            "platPlc": "...",                    // 주소 (도메인 미사용)
            "mgmBldrgstPk": "12345678901234567"  // BigInt — 문자열로 보존
          }
        ]
      }
    }
  }
}
```

ACL:
- `bldNm` → `building_name` (Option)
- `mainPurpsCdNm` 한글 → `BuildingPurposeCode` enum (factory/warehouse/retail/etc — 매핑표 별도)
- `strctCdNm` 한글 → `BuildingStructureCode` enum (steel/concrete/wood/etc)
- `totArea` 문자열 → f64 → `AreaM2`
- `grndFlrCnt`/`ugrndFlrCnt` 문자열 → u8
- `heit` 문자열 → f64 (Option)
- `useAprDay` `YYYYMMDD` → `NaiveDate` (Option)
- `geom`: data.go.kr 응답에 폴리곤 없음 → V-World fetch 후 합성하거나 stub PolygonSrid (도메인은 None 허용 안 함)

**문제**: data.go.kr 건축물대장 응답에 **건물 폴리곤 없음**. `Building.geom: PolygonSrid` 가 *required*. 두 옵션:
- A. 도메인 변경 — `geom: Option<PolygonSrid>` (큰 변화)
- B. 합성 — V-World 의 필지 폴리곤을 building.geom 으로 임시 사용 (정확하지 않음)
- C. 미구현 — `BuildingReader::fetch_by_pnu` 가 아직 polygon 없는 건물 반환 안 함, 별도 fetch with V-World

**선택**: **B (합성)** 임시. 정확한 건물 footprint 는 V-World 의 다른 레이어 (`AL_D194_*` 건물 footprint) 또는 R2 PMTiles 가 필요 — SP4-iii-e 후속. 현재는 *필지 폴리곤 = 건물 폴리곤* approximation 으로 진행.

→ `DataGoKrBuildingReader.fetch_by_pnu` 가 내부에서 `VWorldClient.fetch_feature_by_pnu` 호출해 폴리곤 받음. 의존:
- `DataGoKrBuildingReader` 가 `Arc<VWorldClient>` 보유

이 합성은 *명확한 trade-off* — README 와 spec 에 명시.

### 3.4 raw_response 보존

- `DataGoKrBuildingReader` 가 `Arc<dyn RawCapture>` 보유
- 응답 받는 즉시 `capture(pnu, "data_go_kr_building", &raw, now)`
- best-effort (실패 시 warn 후 진행, V-World 패턴 동일)

---

## 4. 검증 기준 (DoD)

1. `Policy::data_go_kr_default()` 추가 + 단위 테스트
2. `crates/data-clients/data-go-kr/` 신규 lib
3. `pnu_split` 단위 테스트 (10자리 PNU 분해 검증)
4. `parse_building_title` 단위 테스트 5-7
5. `DataGoKrBuildingReader` impl `BuildingReader::fetch_by_pnu`
6. wiremock 통합 6 시나리오
7. 워크스페이스 members 갱신
8. 3 CI workflow 그린
9. clippy `--all-targets -- -D warnings` 통과
10. SSOT 갱신

---

## 5. SSS 7 기둥

| 기둥 | 적용 |
|---|---|
| 1 일관성 | data.go.kr 도 `circuit_breaker::execute` 통과. `RawCapture` 동일 trait |
| 3 추적성 | raw_response → `parcel_external_data` (source=`data_go_kr_building`) |
| 4 안전성 | timeout 15s, retry 2회. honest failure (5xx 그대로). PNU 19자리 invariant 가 분해 안전 보장 |
| 7 명확성 | building.geom 합성은 README/spec 에 명시 — *추측* 아님 |

---

## 6. Follow-up

- **FU 40**: `Building.geom` 정확한 footprint — SP4-iii-e (V-World AL_D194 또는 R2 PMTiles) 도입 시
- **FU 41**: `mainPurpsCdNm` / `strctCdNm` 한글 → enum 매핑표 (data.go.kr 표준 코드 28+종)
- **FU 42**: `fetch_by_id` 구현 — `mgmBldrgstPk` 문자열 키
- **FU 43**: 캐시 정책 (`expires_at = fetched_at + 30 days` per docs)
- **FU 44**: 토지대장 — SP4-iii-a-2 또는 별도 sub-task

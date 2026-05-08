# 공공데이터포털 (data.go.kr)

## 개요

- 운영 기관: 행정안전부
- 공식 사이트: https://www.data.go.kr
- 우리 사용 후보 API: 건축물대장, 토지대장, 부동산 실거래가, 행정표준코드, 산업단지

## 인증

- 회원가입 → 각 API별 활용신청 → serviceKey 발급
- 환경변수: `ODP_SERVICE_KEY`
- 일부 API는 자동 승인 / 일부는 1-2일 검토

## Rate Limit

- API마다 상이 (보통 일 10,000 호출)
- 상용은 별도 협의
- URL 파라미터 또는 헤더로 키 전달

## 우리 사용 후보 API

| API | 제공기관 | 용도 | 우리 코드 |
|-----|---------|------|----------|
| **건축물대장 (표제부)** | 국토교통부 | 연면적, 건폐율, 용적률, 층수, 구조, 주차, 승강기 | ✅ 라이브 wire 됨 |
| 건축물대장 (총괄/전유부) | 국토교통부 | 집합건물 호실 정보 | ⏸ 후속 |
| 토지대장 | 국토교통부 | 면적, 지목, 소유 구분, 공시지가 | ⏸ 후속 |
| 부동산 실거래가 | 국토교통부 | 거래일, 거래금액, 매매 vs 임대 | ⏸ 일별 배치 검토 |
| 행정표준코드 | 행정안전부 | 법정동·행정동 코드 매핑 | ⏸ |
| 도로명주소 | 행정안전부 | 주소 정규화 | ⏸ |
| 산업단지 입주 정보 | 한국산업단지공단 | 단지별 입주 자격 | ⏸ |

## 요청 예시 (건축물대장 표제부 — 활성)

> ⚠ **endpoint 경로 변경 (FU 41, 2026-05-04)**: `BldRgstService_v2` 는 deprecated.
> 활성 endpoint = `BldRgstHubService`. 이전 endpoint 는 HTTP 200 + body `"Unexpected errors"`
> 텍스트 반환 → JSON parse 실패. 본 문서가 SSOT.

```
GET https://apis.data.go.kr/1613000/BldRgstHubService/getBrTitleInfo
  ?ServiceKey={ODP_SERVICE_KEY}
  &sigunguCd=11680      # PNU chars 0-4
  &bjdongCd=10100       # PNU chars 5-9
  &platGbCd=0           # PNU char 10: "1" 일반 → "0", "2" 산 → "1" (data.go.kr 별 변환)
  &bun=0737             # PNU chars 11-14
  &ji=0000              # PNU chars 15-18
  &numOfRows=100
  &pageNo=1
  &_type=json
```

PNU 19자리 → `sigunguCd` (5) + `bjdongCd` (5) + `platGbCd` (1, 매핑) + `bun` (4) + `ji` (4) 파싱.
구현 참조: [crates/data-clients/data-go-kr/src/pnu_split.rs](../../crates/data-clients/data-go-kr/src/pnu_split.rs).

## 응답 필드 카탈로그 (`getBrTitleInfo` — 80+ 필드)

> 📍 **Ground truth**: 라이브 호출 fixture
> [`live_2026-05-08_gangnam_yeoksam_737.json`](../../crates/data-clients/data-go-kr/tests/fixtures/live_2026-05-08_gangnam_yeoksam_737.json)
> (강남구 역삼동 737 = 강남파이낸스센터, PNU `1168010100107370000`).
> schema drift 발생 시 [`services/api/src/building_reader.rs`](../../services/api/src/building_reader.rs)
> 의 `parse_items_handles_live_fixture` 테스트가 가장 먼저 깨짐.

### 식별자 + 위치

| 필드 | 타입 | 추출 (rich) | 추출 (panel) | 비고 |
|---|---|---|---|---|
| `rnum` | number | ❌ | ❌ | 행 번호 (배열 index, 무시 가능) |
| `platPlc` | string | ❌ | ❌ | **대지 위치** (한글 풀주소) — P3 추출 후보 |
| `sigunguCd` | string | ❌ | ❌ | PNU 에서 derive 가능 |
| `bjdongCd` | string | ❌ | ❌ | PNU 에서 derive 가능 |
| `platGbCd` | string | ❌ | ❌ | PNU 에서 derive 가능 |
| `bun` | string | ❌ | ❌ | PNU 에서 derive 가능 |
| `ji` | string | ❌ | ❌ | PNU 에서 derive 가능 |
| **`mgmBldrgstPk`** | **number** ⚠ | ❌ | ✅ → `String` | docs 가이드 = String 인데 실제 number. parser helper `parse_id_as_string` |
| `regstrGbCd` / `regstrGbCdNm` | string | ❌ | ❌ | 대장구분 (1=일반, 2=집합) |
| `regstrKindCd` / `regstrKindCdNm` | string | ❌ | ❌ | 대장종류 |
| `newPlatPlc` | string | ❌ | ❌ | 도로명대지위치 |
| **`bldNm`** | string | ✅ | ✅ | **건물명** |
| `splotNm` | string | ❌ | ❌ | 특수지명 |
| `block` / `lot` | string | ❌ | ❌ | 블록 / 로트 |
| `bylotCnt` | number | ❌ | ❌ | 외필지수 |

### 도로명주소

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `naRoadCd` | string | ❌ | 도로코드 |
| `naBjdongCd` | string | ❌ | 법정동코드 |
| `naUgrndCd` | string | ❌ | 지상지하코드 |
| `naMainBun` / `naSubBun` | string | ❌ | 도로명건물 본/부번 |

### 동 정보

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `dongNm` | string | ❌ | **동명** — P3 추출 후보 |
| `mainAtchGbCd` / `mainAtchGbCdNm` | string | ❌ | 주부속구분 (0=주, 1=부속) |

### 면적 + 비율 ⭐ 산업 매물 핵심

| 필드 | 타입 | 추출 (rich) | 추출 (panel) | 가치 |
|---|---|---|---|---|
| **`platArea`** | **number** | ❌ | ❌ | **대지면적 m²** ⭐⭐⭐ — P3 1순위 |
| **`archArea`** | **number** | ❌ | ❌ | **건축면적 m²** ⭐⭐⭐ — P3 1순위 |
| **`bcRat`** | **number** | ❌ | ❌ | **건폐율 %** ⭐⭐⭐ — 신축/증축 검토 |
| **`totArea`** | **number** ⚠ | ✅ → `AreaM2` | ✅ → `f64` | **연면적 m²** — parser helper `parse_f64_field` (number/string 둘 다) |
| `vlRatEstmTotArea` | number | ❌ | ❌ | 용적률 산정용 연면적 m² |
| **`vlRat`** | **number** | ❌ | ❌ | **용적률 %** ⭐⭐⭐ — 신축 가능성 |

### 구조 / 용도 / 지붕

| 필드 | 타입 | 추출 (rich) | 추출 (panel) | 비고 |
|---|---|---|---|---|
| `strctCd` / `strctCdNm` | string | ✅ → `BuildingStructureCode` | ❌ | **구조코드** (Cd primary + CdNm fallback) |
| `etcStrct` | string | ❌ | ❌ | 기타구조 |
| `mainPurpsCd` / `mainPurpsCdNm` | string | ✅ → `BuildingPurposeCode` | ✅ (label only) | **주용도** (Cd primary + CdNm fallback) |
| `etcPurps` | string | ❌ | ❌ | 기타용도 |
| `roofCd` / `roofCdNm` | string | ❌ | ❌ | 지붕코드 |
| `etcRoof` | string | ❌ | ❌ | 기타지붕 |

### 가구/세대/호 수

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `hhldCnt` | number | ❌ | 세대수 — 집합건물 분리매물 시 P3 후보 |
| `fmlyCnt` | number | ❌ | 가구수 |
| `hoCnt` | number | ❌ | 호수 |

### 층/높이

| 필드 | 타입 | 추출 (rich) | 추출 (panel) | 비고 |
|---|---|---|---|---|
| `heit` | number | ✅ → `Option<f64>` | ❌ | **높이 m** |
| `grndFlrCnt` | number | ✅ → `u8` | ❌ | **지상층수** |
| `ugrndFlrCnt` | number | ✅ → `u8` | ❌ | **지하층수** |

### 승강기 ⭐ 산업 매물 검토

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `rideUseElvtCnt` | number | ❌ | 승용 승강기수 — P3 후보 |
| `emgenUseElvtCnt` | number | ❌ | 비상용 승강기수 |

### 부속건축물 ⭐ 창고+사무동 패턴

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `atchBldCnt` | number | ❌ | 부속건축물수 |
| `atchBldArea` | number | ❌ | 부속건축물 면적 m² |
| `totDongTotArea` | number | ❌ | 총 동 연면적 |

### 주차장 ⭐⭐ 물류 매물 critical

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `indrMechUtcnt` | number | ❌ | 옥내 기계식 대수 |
| `indrMechArea` | number | ❌ | 옥내 기계식 면적 m² |
| `oudrMechUtcnt` | number | ❌ | 옥외 기계식 대수 |
| `oudrMechArea` | number | ❌ | 옥외 기계식 면적 m² |
| `indrAutoUtcnt` | number | ❌ | 옥내 자주식 대수 |
| `indrAutoArea` | number | ❌ | 옥내 자주식 면적 m² |
| `oudrAutoUtcnt` | number | ❌ | 옥외 자주식 대수 |
| `oudrAutoArea` | number | ❌ | 옥외 자주식 면적 m² |

### 날짜 (모두 `YYYYMMDD` string, 빈 값 = `" "`)

| 필드 | 타입 | 추출 (rich) | 추출 (panel) | 비고 |
|---|---|---|---|---|
| `pmsDay` | string | ❌ | ❌ | **허가일** — 개발 timeline |
| `stcnsDay` | string | ❌ | ❌ | **착공일** |
| `useAprDay` | string | ✅ → `NaiveDate` | ✅ → `Option<String>` | **사용승인일** (8자리 검증) |
| `crtnDay` | string | ❌ | ❌ | 생성일 |

### 허가번호

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `pmsnoYear` | string | ❌ | 허가번호 연도 |
| `pmsnoKikCd` / `pmsnoKikCdNm` | string | ❌ | 허가번호 기관코드 |
| `pmsnoGbCd` / `pmsnoGbCdNm` | string | ❌ | 허가번호 구분코드 |

### 인증 / 등급

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `engrGrade` | string | ❌ | 에너지효율등급 |
| `engrRat` | number | ❌ | 에너지효율인증 점수 |
| `engrEpi` | number | ❌ | EPI 점수 |
| `gnBldGrade` | string | ❌ | 친환경건축물등급 |
| `gnBldCert` | number | ❌ | 친환경건축물인증 점수 |
| `itgBldGrade` | string | ❌ | 지능형건축물등급 |
| `itgBldCert` | number | ❌ | 지능형건축물인증 점수 |

### 내진

| 필드 | 타입 | 추출 | 비고 |
|---|---|---|---|
| `rserthqkDsgnApplyYn` | string | ❌ | 내진설계적용여부 (`Y`/`N`) |
| `rserthqkAblty` | string | ❌ | 내진능력 |

## ⚠ 타입 quirk — number/string 가이드 위반

docs 가이드 ("BigInt 는 String 으로") 와 **실 응답이 다름**. 라이브 fixture 검증 결과:

| 필드 | docs 가이드 | 실제 응답 | 우리 처리 |
|---|---|---|---|
| `mgmBldrgstPk` | String 권장 (BigInt 회피) | **number** (`1024112777`) | `parse_id_as_string` — number/string 둘 다 → `String` |
| `totArea` | (명시 없음) | **number** (`212615.29`) | `parse_f64_field` — number/string 둘 다 → `f64` |
| `platArea` / `archArea` / `bcRat` / `vlRat` / `heit` | (명시 없음) | **number** | rich parser 의 `read_f64_field` 동일 처리 |

**모든 수치 필드는 number 가정이 맞고, string 인 경우만 fallback** — 새 필드 추가 시 본 helper 사용.

## 데이터 손실률

| Layer | 추출 / 가용 |
|---|---|
| Rich `Building` (parser.rs) | **8 / 80** = 10% |
| Panel `BuildingItem` (api/building_reader.rs) | **5 / 80** = 6% |
| **Bronze `raw_response JSONB`** | **80 / 80** = **100% ✅** |

→ **재추출 가능**: raw_response 가 100% 보존되므로 미래 SQL 한 줄로 어떤 필드든 추출 가능 (`raw_response->>'platArea'`).

## Circuit Breaker 정책

- timeout: 15초
- retry: 2회 (지수 백오프 1초/2초/4초)
- fallback: cached response
- 구현: [crates/circuit-breaker](../../crates/circuit-breaker/) `Policy::data_go_kr_default`

## 캐시 정책

| 종류 | TTL |
|------|-----|
| 건축물대장 | 30일 (변경 빈도 낮음) |
| 토지대장 | 30일 |
| 실거래가 | 일별 배치 ingest (API 호출 안 함) |
| 행정표준코드 | 90일 |

## 라이선스 (각 데이터셋별)

- 각 API의 **이용허락범위** 필드 확인 필수
- 보통 출처 표기 시 자유 사용
- 일부 데이터셋은 가공·재배포 제한

## 사용자 노출 정책

- ✅ 데이터 그대로 표시 (출처: 국토교통부 / 데이터별 표기)
- ❌ LLM 가공 텍스트 (옵션 A 준수)

## raw 보존 (Bronze)

```sql
create table parcel_external_data (
    pnu char(19),
    source varchar(40) check (source in (
        'vworld',
        'data_go_kr_building',
        'data_go_kr_land',
        'data_go_kr_realtransaction',
        'korean_law'
    )),
    raw_response jsonb not null,
    fetched_at timestamptz not null,
    expires_at timestamptz,
    primary key (pnu, source)
);
```

- `source = 'data_go_kr_building'` (`getBrTitleInfo` 응답)
- 마이그레이션: [migrations/30006_parcel_external_data.sql](../../migrations/30006_parcel_external_data.sql)
- DB 구현체: [crates/db/src/raw_capture.rs](../../crates/db/src/raw_capture.rs) `PgRawCapture`
- wire: [services/api/src/main.rs](../../services/api/src/main.rs) — `parcel_lookup` (V-World) 가 `PgRawCapture` 주입 받음.
  data.go.kr 측 wire 는 `BuildingRegisterReader` 라이브 reader 추가 시 동일 패턴 (P4 후속).

## 에이전트 경로 (참고)

`opendata-mcp` (ceami): Apache-2.0. 3개 도구 (`search_api`, `get_std_docs`, `fetch_data`). 개발자 Claude 세션에서 *API 발견* 용도.

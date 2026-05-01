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

| API | 제공기관 | 용도 |
|-----|---------|------|
| 건축물대장 (표제부) | 국토교통부 | 연면적, 건폐율, 용적률, 층수, 구조 |
| 건축물대장 (총괄/전유부) | 국토교통부 | 집합건물 호실 정보 |
| 토지대장 | 국토교통부 | 면적, 지목, 소유 구분, 공시지가 |
| 부동산 실거래가 | 국토교통부 | 거래일, 거래금액, 매매 vs 임대 |
| 행정표준코드 | 행정안전부 | 법정동·행정동 코드 매핑 |
| 도로명주소 | 행정안전부 | 주소 정규화 |
| 산업단지 입주 정보 | 한국산업단지공단 | 단지별 입주 자격 |

## 요청 예시 (건축물대장 표제부)

```
GET https://apis.data.go.kr/1613000/BldRgstService_v2/getBrTitleInfo?
  ServiceKey={ODP_SERVICE_KEY}
  &sigunguCd=11680
  &bjdongCd=10100
  &platGbCd=0
  &bun=0001
  &ji=0000
  &numOfRows=100
  &pageNo=1
  &_type=json
```

PNU 19자리 → `sigunguCd` (3) + `bjdongCd` (5) + `platGbCd` (1) + `bun` (4) + `ji` (4) 파싱.

## Circuit Breaker 정책

- timeout: 15초
- retry: 2회 (지수 백오프 1초/2초/4초)
- fallback: cached response

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

## BigInt 주의 (v2 안티패턴 회피)

`mgmBldrgstPk` 같은 큰 숫자는 **문자열로 저장**:

```rust
// crates/data-clients/data-go-kr/types.rs
#[derive(serde::Deserialize)]
pub struct BuildingTitleResponse {
    #[serde(rename = "mgmBldrgstPk")]
    pub mgm_bldrgst_pk: String,  // ✅ String (NOT i64 — JSON 직렬화 시 과학표기법 손실 위험)
    // ...
}
```

## 사용자 노출 정책

- ✅ 데이터 그대로 표시 (출처: 국토교통부 / 데이터별 표기)
- ❌ LLM 가공 텍스트 (옵션 A 준수)

## raw 보존

```sql
create table parcel_external_data (
    pnu char(19),
    source varchar(20) check (source in ('data_go_kr_building', 'data_go_kr_land', 'data_go_kr_realtransaction', ...)),
    raw_response jsonb not null,
    fetched_at timestamptz not null,
    expires_at timestamptz not null,
    primary key (pnu, source)
);
```

## 에이전트 경로 (참고)

`opendata-mcp` (ceami): Apache-2.0. 3개 도구 (`search_api`, `get_std_docs`, `fetch_data`). 개발자 Claude 세션에서 *API 발견* 용도.

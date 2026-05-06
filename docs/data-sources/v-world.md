# V-World 공간정보 API

> **이 문서의 SSOT 보증** — 본 문서의 모든 응답 예시는 실 V-World 호출에서
> 캡처된 fixture 와 1:1 대응. fixture: [crates/data-clients/vworld/tests/fixtures/](../../crates/data-clients/vworld/tests/fixtures/).
> 문서가 갱신되려면 fixture 도 같이 갱신되어야 함 (CI 가 강제 — ADR 0015 참조).

## 개요

- 운영 기관: 국토교통부 산하 공간정보산업진흥원
- 공식 사이트: <https://www.vworld.kr>
- 개발자 센터: <https://www.vworld.kr/dev/v4api.do>
- 우리 프로젝트 핵심 데이터 소스 (필지/건축물/지구단위계획/도시계획시설/법적지정 42종)
- ACL crate: [`crates/data-clients/vworld/`](../../crates/data-clients/vworld/)

## 인증

- 회원가입 → 애플리케이션 등록 → API 키 발급
- **도메인 1개 등록 필수** (Referer 검증)
- 환경변수: `VWORLD_API_KEY`, `VWORLD_DOMAIN`
- 등록 도메인이 `http://localhost:3000` 이면 `VWORLD_DOMAIN=localhost:3000` 으로 설정 (포트 포함)
- 키 등급: **개발키** (운영 도메인 거부, 6개월 만료, 3회 연장 가능) / **운영키** (별도 심사)

## 핵심 레이어

각 레이어가 가진 attribute 와 사용 가능한 filter 가 다름. **PNU 필터링 가능 여부**
가 가장 중요 — 이걸 모르면 [`INVALID_RANGE`] 에러.

| 레이어 ID | 설명 | PNU `attrFilter` | 사용처 |
|----------|------|---|---|
| `LP_PA_CBND_BUBUN` | **연속지적도 (필지 경계)** | ✅ | `VWorldParcelReader::fetch_by_pnu` (주 SSOT) |
| `LT_C_UQ111` | 도시지역 용도지역 | ❌ (`uname/dyear/...` 만) | spatial intersect (`geomFilter`) 별도 호출 |
| `LT_C_UQ112` | 관리지역 | ❌ | spatial intersect |
| `LT_C_UQ113` | 농림지역 | ❌ | spatial intersect |
| `LT_C_UQ114` | 자연환경보전지역 | ❌ | spatial intersect |
| `UPISUQ161` | 지구단위계획 | (확인 필요) | spatial intersect |
| `UPISUQ171` | 개발제한구역 | (확인 필요) | spatial intersect |

> ⚠️ **함정**: `LT_C_UQ111` 가 직관적으로는 "필지의 용도지역"이지만, 본 레이어는
> PNU attribute 를 가지지 않음. PNU 로 용도지역을 알려면 먼저
> `LP_PA_CBND_BUBUN` 으로 필지 geometry 를 받고, 그 geometry 로 `LT_C_UQ111` 에
> spatial filter 호출하는 2-step 패턴 필요.

[`INVALID_RANGE`]: #응답--에러-status-error

## 요청 — 단일 필지 조회 (PNU 기반)

```
GET https://api.vworld.kr/req/data
    ?service=data
    &request=GetFeature
    &data=LP_PA_CBND_BUBUN
    &key={VWORLD_API_KEY}
    &domain={VWORLD_DOMAIN}
    &attrFilter=pnu:=:{PNU_19자리}
    &format=json
    &size=10
    &geometry=true
    &crs=EPSG:4326
```

**구현**: [`VWorldClient::fetch_feature_by_pnu`](../../crates/data-clients/vworld/src/client.rs)

## 응답 — 정상 (`status: "OK"`)

Fixture: [`real_parcel_boundary_gangnam_yeoksam_737.json`](../../crates/data-clients/vworld/tests/fixtures/real_parcel_boundary_gangnam_yeoksam_737.json) (강남파이낸스, PNU `1168010100107370000`)

```json
{
  "response": {
    "service": { "name": "data", "version": "2.0", "operation": "GetFeature", "time": "31(ms)" },
    "status": "OK",
    "record": { "total": "1", "current": "1" },
    "page": { "total": "1", "current": "1", "size": "10" },
    "result": {
      "featureCollection": {
        "type": "FeatureCollection",
        "bbox": [127.03582619570822, 37.49914715315316, 127.03740886415942, 37.500495637519585],
        "features": [{
          "type": "Feature",
          "geometry": {
            "type": "MultiPolygon",
            "coordinates": [[[[127.0358, 37.5001], ...]]]
          },
          "properties": {
            "gosi_year": "2025",
            "pnu": "1168010100107370000",
            "jibun": "737 대",
            "bonbun": "737",
            "bubun": "",
            "addr": "서울특별시 강남구 역삼동 737",
            "gosi_month": "01",
            "jiga": "67300000"
          },
          "id": "LP_PA_CBND_BUBUN.826412"
        }]
      }
    }
  }
}
```

### `LP_PA_CBND_BUBUN` properties — 매핑 표

| 응답 필드 | 도메인 매핑 | 비고 |
|---|---|---|
| `pnu` | `Parcel.pnu` | 19자리 |
| `jibun` ("737 대") | `Parcel.jibun_address` (addr 우선) + `Parcel.land_use_type` (마지막 토큰) | 본 레이어엔 별도 `lndcgr_nm` 없음 |
| `bonbun` / `bubun` | (미사용 — 필요 시 향후) | |
| `addr` | `Parcel.jibun_address` (풀주소) | |
| `jiga` (₩/m²) | `Parcel.official_land_price_per_m2` | 0 또는 누락이면 `None` (도로 등 미고시) |
| `gosi_year` / `gosi_month` | `Parcel.gosi_year_month` (`Some` ↔ `jiga` `Some`) | 공시지가 lineage |
| `geometry` | `Parcel.geom: MultiPolygonSrid` | 항상 `MultiPolygon` |

**미제공** (다른 소스로 채워야 함):
- 도로명 주소 (`Parcel.road_address`) — NSDI 별도 API 또는 V-World 주소 API
- 면적 (`Parcel.area`) — PostGIS `ST_Area` 또는 건축물대장 (data.go.kr)
- 용도지역 (`Parcel.zoning`) — `LT_C_UQ11{1..4}` spatial intersect 별도 호출

## 응답 — 결과 없음 (`status: "NOT_FOUND"`)

Fixture: [`real_parcel_boundary_not_found.json`](../../crates/data-clients/vworld/tests/fixtures/real_parcel_boundary_not_found.json)

```json
{
  "response": {
    "service": { "name": "data", "version": "2.0", "operation": "GetFeature" },
    "status": "NOT_FOUND",
    "record": { "total": "0", "current": "0" },
    "page": { "total": "1", "current": "1", "size": "10" }
  }
}
```

> ⚠️ **`result` 필드 자체가 없음** — `OK` 상태 가정으로 `result.featureCollection`
> 직접 접근하면 NPE-equivalent. envelope 파서가 status 분기 후 처리.

## 응답 — 에러 (`status: "ERROR"`)

Fixture: [`real_error_invalid_range.json`](../../crates/data-clients/vworld/tests/fixtures/real_error_invalid_range.json)

```json
{
  "response": {
    "service": { "name": "data", "version": "2.0", "operation": "GetFeature" },
    "status": "ERROR",
    "error": {
      "level": "1",
      "code": "INVALID_RANGE",
      "text": "attrFilter 파라미터의 값이 유효한 범위를 넘었습니다. 유효한 파라미터 값의 범위 : 속성명은 [uname,dyear,dnum,sido_name,sigg_name,ag_geom] 중 하나를 입력하십시오., 입력한 파라미터 값 : pnu:=:1168010100107370000"
    }
  }
}
```

### 흔한 에러 코드

| 코드 | 원인 | 대응 |
|---|---|---|
| `INVALID_RANGE` | `attrFilter` 가 해당 레이어 attribute 와 불일치 | 레이어 attribute 표 확인 (위) |
| `INVALID_KEY` | 키 또는 등록 도메인 mismatch | `VWORLD_API_KEY`/`VWORLD_DOMAIN` 확인 |
| `NO_PERMISSION` | 키 등급(개발/운영)으로 차단된 레이어 | 운영키 신청 |
| `OVER_LIMIT` | 일일 호출 한도 초과 (운영키만) | rate-limit + 캐시 hit 율 확인 |

**도메인 매핑**: `ParseError::VWorldApi { code, text }` ([error.rs](../../crates/data-clients/vworld/src/error.rs)). 호출자는 코드별 분기 가능.

## 지오코딩 (주소 → 좌표)

```
GET https://api.vworld.kr/req/address
    ?service=address&request=getCoord&type=ROAD
    &address=서울특별시 강남구 테헤란로 123
    &key={VWORLD_API_KEY}&format=json&crs=EPSG:4326
```

(별도 client 미구현. 필요 시 추가.)

## Circuit Breaker 정책

[`Policy::vworld_default`](../../crates/circuit-breaker/src/policy.rs):

- timeout: 10초
- retry: 1회 (지수 백오프 1초/2초)
- circuit open 조건: 5초 내 5번 실패 → 30초 차단
- fallback: 호출자 책임 (cached response 또는 honest failure)

## 라이선스 / 재배포

- 데이터: 공공저작물 (CC BY 또는 V-World 이용약관)
- **재배포 가능 데이터**: 용도지역, 도시계획 (출처 표기 필수)
- **재배포 제한 데이터**: 일부 상세 (확인 필요 — 데이터셋별)
- 우리 정책: **모든 노출 화면에 "출처: V-World" 표기**

## 프로덕션 사용 주의

- [ ] API 키 서버 사이드만 (Next.js Server Component / Rust)
- [ ] 도메인 Referer 우회용 서버 프록시 필수 (브라우저 직접 호출 X)
- [ ] 응답 캐시 (Redis TTL 24h, raw_response JSONB 보존)
- [ ] 쿼터 관측 (OTel 메트릭 + Sentry 알림)
- [ ] **Honest failure** — 5xx/ERROR 를 가짜 데이터로 덮지 말 것 (`ParseError::VWorldApi` 그대로 전파)

## raw_response 보존 (SSOT 보호)

```sql
-- crates/db에서 정의
create table parcel_external_data (
    pnu char(19) not null,
    source varchar(20) not null check (source in ('vworld', ...)),
    raw_response jsonb not null,
    fetched_at timestamptz not null,
    expires_at timestamptz not null,
    primary key (pnu, source)
);
```

→ 1년 후에도 *원본 그대로* 재현 가능 (감사·분쟁 시 증빙).

## Drift 검출

- **Unit test**: [`tests/fixtures/real_*.json`](../../crates/data-clients/vworld/tests/fixtures/) 가
  파서 입력. fixture는 실 호출 캡처 — hand-crafted 금지 (ADR 0015 R1)
- **Smoke test**: `cargo test -p vworld-client --features real-api --test smoke_real_api -- --ignored`
  — CI nightly cron ([`.github/workflows/api-drift-smoke-test.yml`](../../.github/workflows/api-drift-smoke-test.yml))
- 응답 schema 변경 → smoke test fail → fixture 갱신 + 파서 갱신 필수

## 비용

- 무료 (Phase 1-3)
- 사용자 10K+ 시 일일 쿼터 초과 가능 → 캐시 hit ratio 90%+ 유지 필수
- 상용 플랜은 Phase 4+ 검토

## 에이전트 경로 (참고)

`korean-land-mcp` (UrbanWatcherKr/korean-land-mcp): MIT 라이선스, V-World 래퍼
7개 도구. 메인 코드 import 금지, `reference/` 학습용 + 개발자 Claude 세션 전용.

## 변경 이력

- **2026-05-06** — 실 API 응답 기준 전면 재작성. 옛 문서가 `LT_C_UQ111` 를 PNU
  조회 레이어로 잘못 표기 → 실 API에서 `INVALID_RANGE` 에러. 정정 + envelope
  status 분기 + MultiPolygon geometry 명시. 상세는 [ADR 0015](../adr/0015-v-world-acl-rearchitecture.md).

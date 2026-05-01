# V-World 공간정보 API

## 개요

- 운영 기관: 국토교통부 산하 공간정보산업진흥원
- 공식 사이트: https://www.vworld.kr
- 개발자 센터: https://www.vworld.kr/dev/v4api.do
- 우리 프로젝트 핵심 데이터 소스 (필지·용도지역·건축물·지구단위계획·도시계획시설·법적지정 42종)

## 인증

- 회원가입 → 애플리케이션 등록 → API 키 발급
- **도메인 1개 등록 필수** (Referer 검증)
- 환경변수: `VWORLD_API_KEY`, `VWORLD_DOMAIN`
- Phase 1 개발: `VWORLD_DOMAIN=localhost`

## Rate Limit / 쿼터

- 무료 (요청 수 일일 한도, 플랜별 상이)
- 상용 서비스는 별도 문의
- 남용 시 계정 차단 가능

## 핵심 레이어 (`crates/data-clients/vworld/constants.rs` SSOT)

| 레이어 ID | 설명 |
|----------|------|
| `LT_C_UQ111` | 도시지역 용도지역 |
| `LT_C_UQ112` | 관리지역 |
| `LT_C_UQ113` | 농림지역 |
| `LT_C_UQ114` | 자연환경보전지역 |
| `UPISUQ161` | 지구단위계획 |
| `UPISUQ171` | 개발제한구역 |
| `UPISUQ151-159` | 도시계획시설 9종 |
| `LT_C_*` (42종) | 농업/임업/산업단지/환경/문화재/항공 등 법적지정 |

## 요청 예시 (Rust + reqwest)

### 용도지역 조회 (WFS GetFeature)

```
GET https://api.vworld.kr/req/data?
  service=data
  &request=GetFeature
  &data=LT_C_UQ111
  &key={VWORLD_API_KEY}
  &domain={VWORLD_DOMAIN}
  &geomFilter=POINT(127.0276 37.4979)
  &format=json
  &size=10
  &geometry=true
  &crs=EPSG:4326
```

### 지오코딩 (주소 → 좌표)

```
GET https://api.vworld.kr/req/address?
  service=address
  &request=getCoord
  &type=ROAD
  &address=서울특별시 강남구 테헤란로 123
  &key={VWORLD_API_KEY}
  &format=json
  &crs=EPSG:4326
```

## Circuit Breaker 정책

- timeout: 10초
- retry: 1회 (지수 백오프 1초/2초)
- circuit open 조건: 5초 내 5번 실패 → 30초 차단
- fallback: cached response (TTL 24h) 또는 honest failure

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
- [ ] **Honest failure** — 5xx 에러를 가짜 데이터로 덮지 말 것

## raw_response 보존

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

## 비용

- 무료 (Phase 1-3)
- 사용자 10K+ 시 일일 쿼터 초과 가능 → 캐시 hit ratio 90%+ 유지 필수
- 상용 플랜은 Phase 4+ 검토

## 에이전트 경로 (참고)

`korean-land-mcp` (UrbanWatcherKr/korean-land-mcp): MIT 라이선스, V-World 래퍼 7개 도구. 메인 코드 import 금지, `reference/` 학습용 + 개발자 Claude 세션 전용.

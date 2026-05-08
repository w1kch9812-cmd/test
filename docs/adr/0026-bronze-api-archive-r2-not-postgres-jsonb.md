# ADR 0026 — Bronze (외부 API 응답 raw archive) → R2, Postgres jsonb 폐기

| | |
|---|---|
| 작성일 | 2026-05-08 |
| 상태 | Accepted |
| 결정자 | Claude + 사용자 (architecture pushback) |
| 선행 | [0016 Medallion Base Layer (PMTiles)](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) |
| Supersedes (부분) | `parcel_external_data.raw_response JSONB` 컬럼 사용 (migration 30006 의 일부) |

## 결정

외부 API raw 응답 (V-World, data.go.kr 등) 의 **Bronze 보존을 Postgres jsonb 가 아닌 R2 (S3-호환 객체 저장소) 로 이전**.

키 구조:
```
{R2_BUCKET}/bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json
```

예: `gongzzang/bronze/data_go_kr_building/2026/05/08/1168010100107370000_1715156234567.json`

## 컨텍스트

기존 [migration 30006](../../migrations/30006_parcel_external_data.sql) 가 raw 응답을 `parcel_external_data.raw_response JSONB` 에 (pnu, source) PK 로 UPSERT 보존. 산업 부동산 SSS-grade scope (전국 ~40M 필지 / ~7M 건축물 + 시계열) 에서 다음 한계 노출:

### 1. 비용 — Postgres 가 R2 의 ~7-10x

| 항목 | Postgres (RDS) | R2 |
|---|---|---|
| 1.5TB 저장 | $0.10/GB/월 = $150/월 + 백업 multiplier (~3x) → **~$450/월** | $0.015/GB/월 = **~$22.50/월** |
| egress | RDS → app server 같은 AZ 면 무료, cross-region 이면 $0.02/GB | **무료** (R2 의 핵심 가치) |
| 인덱스 | jsonb GIN ~30% 추가 storage + vacuum 비용 | n/a |
| backup | RDS 자동 backup × N일 = storage × N | bucket 레벨 versioning, copy 비용 0 |

### 2. 의미론적 mismatch

- **UPSERT** = 같은 `(pnu, source)` 재호출 → 옛 raw **영구 손실**. "감사 / 분쟁 시 *그 시점* 응답 재현" SSS 조건 위반
- 정부 공공 API 는 *시간이 지나면서 같은 PNU 응답이 바뀜* (건축물 증축, 공시지가 갱신 등) — 시계열 보존이 도메인 가치
- R2 의 timestamped 키 = 진짜 append-only Bronze

### 3. 운영 부담

- jsonb GIN 인덱스 vacuum / bloat
- Postgres connection pool 이 **raw 적재 트래픽** 과 **사용자 read 트래픽** 공유 → 대규모 ingest 시 read 영향
- R2 = 분리된 control plane, app DB 부담 0

### 4. 사용자 architectural pushback

> "API 응답 다 받으면 데이터양이 엄청 클 텐데 그걸 RDS 에 넣자고?"

→ 본 결정의 직접적 trigger.

## 대안

- **대안 1 — 현 Postgres jsonb 유지**: 단기는 단순 (이미 wired). 장기는 cost + UPSERT 손실로 SSS 박탈. 채택 X.
- **대안 2 — DynamoDB / Athena**: 시계열 query 강하지만 운영 추가 (AWS 의존성 ↑, 학습 곡선). R2 가 우리 stack 에 이미 있음. 채택 X.
- **대안 3 — Postgres + R2 dual-write (hybrid)**: hot (Postgres jsonb UPSERT) + cold (R2 append). 정합성 책임 코드 ↑, 두 곳에서 진실 (SSOT 위반). 채택 X.
- **대안 4 (채택) — R2 only Bronze**: append-only timestamped 키. Postgres 는 *Silver* (parsed entity) + *fetch metadata* (last fetched ts, R2 키 reference) 만.

## 결과

### 긍정

- 비용 ~7-10x 절감 (1.5TB 시 월 $400+ 절약)
- 진짜 append-only 시계열 archive (감사 / 분쟁 / drift 검출)
- Postgres connection pool ingest 부담 0
- bucket-level versioning + lifecycle policy (Cloudflare 자체 기능)

### 부정

- *현재 응답* 빠른 query 불가 — Silver (parsed entity) 가 그 역할 책임 (이미 존재: `Building`, `Parcel` etc)
- R2 outage 시 fetch path 차단 — circuit breaker + best-effort capture (현 코드는 raw_capture 실패 시 panel 응답 정상 진행, SSOT 보호)
- 드물게 raw 분석 필요 시: R2 → `aws s3 cp` + jq, 또는 Athena / DuckDB ad-hoc 쿼리

### 영향 받는 영역

- `crates/data-clients/raw-capture/` — `RawCapture` trait 그대로 (구현체만 추가)
- `services/api/src/` — `R2RawCapture` 신규 + `PgRawCapture` 폐기 wire
- `migrations/` — `parcel_external_data.raw_response jsonb` 컬럼 deprecate (forward-only ALTER 로 nullable 화 + 새 `r2_object_key` 컬럼 추가, 향후 migration 으로 컬럼 자체 drop 가능)
- `docs/data-sources/data-go-kr.md` — Bronze 섹션 갱신 (R2 키 구조 박제)
- `docs/data-sources/v-world.md` — 동일

## 재검토 트리거

- R2 outage 가 분기 1회 초과 → SLA 영향 분석, hybrid 검토
- 일일 PUT 100k+ → R2 class A operation 비용 ($0.36/M) 재검토
- 라이브 query 빈도 ↑ → Athena 도입 검토

## 참조

- → [`0016 Medallion Base Layer`](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) (정적 폴리곤 archive 의 R2 패턴 선례)
- → [`0022 Bronze Scraping Isolated Python Service`](./0022-bronze-scraping-isolated-python-service.md) (V-World dtmk 273 SHP zip → R2 선례)
- → `crates/data-clients/raw-capture/src/lib.rs` (`RawCapture` trait)
- → `services/etl-base-layer/src/r2_upload.rs` (S3 client + R2 endpoint pattern)
- → `migrations/30006_parcel_external_data.sql` (deprecate 대상)

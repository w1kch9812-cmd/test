# services/data-pipeline

Rust ETL 파이프라인. 공공 API → 정제 → DB.

## 의존
- `crates/data-clients/*`, `crates/db`, `crates/observability`
- AWS S3 (raw 백업)

## 정책
- raw 데이터 *항상* 보존 (S3 + DB raw_response JSONB)
- 변환 단계 명시적 (raw → normalized → indexed)
- Replay 가능 (raw에서 재처리)
- 실패한 파이프라인은 알림 + 이전 단계로 자동 롤백
- PostGIS SRID 강제 (4326)

## 주요 파이프라인 (sub-project 9+)
- `vworld-parcel-sync` — V-World 필지 데이터 (행정구역별)
- `data-go-kr-building` — 건축물대장 (PNU별)
- `realtransaction-daily` — 실거래가 일별 신규분
- `industrial-complex-quarterly` — 산업단지 분기별
- `manufacturer-ingest` — 제조업체 (Phase 3+)
- `law-cache-update` — 법령 변경 감지 시

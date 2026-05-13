# services/worker

Rust 배치/크론 작업 (Tokio + apalis).

## 의존
- `crates/domain/*`, `crates/data-clients/*`, `crates/db`, `crates/cache`, `crates/observability`
- Postgres advisory lock (분산 락 — ShedLock 패턴)

## 정책
- 작업당 멱등성 보장
- 실패 시 자동 재시도 + DLQ (Dead Letter Queue)
- 작업 시작/완료/실패 모두 audit log
- 외부 API 호출은 Circuit Breaker

## 작업 (sub-project 9+)
| 작업 | 주기 |
|------|-----|
| `vworld-cache-refresh` | 일일 03:00 — 인기 필지 재갱신 |
| `realprice-ingest` | 일일 04:00 — data.go.kr 실거래 신규분 |
| `building-register-sync` | 주간 일요일 02:00 |
| `cache-expire-sweep` | 시간당 |
| `audit-log-archive` | 월 1일 — R2 archive |
| `embedding-rebuild` | 일일 (Phase 3+) — 매물 임베딩 갱신 |

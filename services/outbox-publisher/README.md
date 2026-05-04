# services/outbox-publisher

공짱 outbox 이벤트 발행 daemon. SP4-i.

## 환경변수

| 변수 | 기본 | 설명 |
|---|---|---|
| `DATABASE_URL` | (필수) | `Postgres` 접속 문자열 |
| `OUTBOX_POLL_INTERVAL_MS` | `1000` | tick 주기 (ms) |
| `OUTBOX_BATCH_SIZE` | `100` | tick 당 fetch limit |
| `RUST_LOG` | `info` | `tracing-subscriber` env filter |

## 기동

```bash
cargo run -p outbox-publisher-service
```

## 종료

`SIGTERM` (Unix) / `Ctrl+C` 로 graceful shutdown — 진행 중 tick 완료 후 종료.

## 발행 대상

v1 의 default sink 는 `LoggingSink` — `tracing::info!` 로 구조화 event 발행해요
(target = `outbox.publish`). 운영 시 `Loki` / `Grafana` 가 해당 target 필터로 발행
흐름 모니터링.

진짜 외부 시스템 (`Kafka` / `Webhook` / `SQS` 등) 통합은 후속 sub-project 에서
같은 `Sink` trait 구현체로 추가해요.

## 후속 (SP4-i 미포함)

- 분산 락 (`SELECT FOR UPDATE SKIP LOCKED` 또는 advisory lock) — 멀티 인스턴스 시
- 외부 sink 구현체 (Kafka / Webhook / SQS / NATS)
- 재시도 정책 (`attempt_count` 컬럼 + DLQ)
- Circuit breaker 통합
- Prometheus metrics

상세: [`docs/superpowers/specs/2026-05-04-sub-project-4-i-outbox-publisher-design.md`](../../docs/superpowers/specs/2026-05-04-sub-project-4-i-outbox-publisher-design.md) § 11.

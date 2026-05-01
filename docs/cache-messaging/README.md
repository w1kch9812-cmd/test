# cache-messaging/

캐싱·메시징·이벤트 SSOT.

## 책임 영역
- L1 인메모리 (moka, Rust)
- L2 분산 (Valkey)
- 메시지 큐 (AWS SQS + SNS)
- 이벤트 스트림 (Kafka/MSK, Phase 4+)
- 이벤트 버스 (EventBridge)
- 분산 락 (Postgres advisory lock)
- Outbox 패턴 (DB ↔ 메시징 일관성)
- 작업 큐 (apalis Rust)

## 작성 예정 문서 (sub-project 4-7)
- `moka-l1.md` — 인메모리 캐시 정책
- `redis-l2.md` — Valkey 클러스터 (실제로는 Valkey)
- `sqs-sns.md` — 비동기 작업 큐
- `eventbridge.md` — 시간/조건 기반 트리거
- `outbox-pattern.md` — Outbox + relay
- `kafka.md` — MSK 사용 (Phase 4+)
- `worker-job-queue.md` — apalis 패턴

## 관련 ADR
- → @docs/adr/0007-cache-moka-valkey.md

## 관련 컨벤션
- → @docs/conventions/rust.md

# backend/

Rust 백엔드 아키텍처·패턴 SSOT.

## 책임 영역
- Axum HTTP 서버 (services/api)
- Tokio worker for Gongzzang-owned asynchronous jobs (`services/worker`)
- DDD Aggregate 17개 (4 Bounded Context)
- Clean Architecture (Port + Adapter)
- CQRS (Read/Write 분리, Phase 3+)
- Event Sourcing (audit-critical 도메인)
- Saga 패턴 (분산 트랜잭션, Phase 3+)
- Circuit Breaker (모든 외부 호출)
- Idempotency (모든 쓰기 요청)
- Outbox 패턴

## 작성 예정 문서 (sub-project 5+)
- `axum.md` — 라우팅 + 미들웨어
- `sqlx.md` — compile-time SQL 패턴
- `ddd-aggregate.md` — 4 BC + 17 Aggregate 설계
- `clean-architecture.md` — Port/Adapter
- `cqrs.md` — Read/Write 모델 (Phase 3+)
- `event-sourcing.md` — audit-critical 도메인
- `saga.md` — 분산 트랜잭션
- `circuit-breaker.md` — failsafe-rs / tower
- `idempotency.md` — Idempotency-Key 헤더
- `outbox-pattern.md` — DB ↔ 메시징

## 관련 ADR
- → @docs/adr/0001-language-rust-ts.md
- → @docs/adr/0002-monorepo-cargo-pnpm-turbo.md

## 관련 컨벤션
- → @docs/conventions/rust.md
- → @docs/conventions/error-format.md

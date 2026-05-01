# crates/domain/core

Core Bounded Context — 핵심 자원 도메인.

## Aggregates
- **User** — 가입, 로그인 (Zitadel 연동), 사업자 검증, RBAC
- **Listing** — 매물 등록·검수·상태 머신
- **Parcel** — 필지 (V-World 캐시, PNU 식별)
- **Building** — 건축물대장 (data.go.kr)
- **IndustrialComplex** — 산업단지
- **Manufacturer** — 제조업체

## 의존
- `crates/domain/shared-kernel` — 공유 값 객체 (Pnu, Money, Geometry)
- 외부 의존 0 (도메인 순수성)

## 정책
- 모든 식별자 = ULID + prefix (`usr_`, `lst_`, ...)
- 값 객체(Newtype) 강제 (Pnu, BusinessNumber, Money 등)
- Aggregate Root만 외부 노출
- Repository = trait (구현은 `crates/db`)
- 도메인 이벤트 = `events.rs`
- 에러 = thiserror enum
- 상태 머신 = enum + match exhaustive

→ ADR-0001, → @docs/conventions/rust.md

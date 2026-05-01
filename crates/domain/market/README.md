# crates/domain/market

Market Bounded Context — 시장·거래 도메인.

## Aggregates
- **RealTransaction** — 국토부 실거래가 (Read-only ingestion)
- **CourtAuction** — 법원 경매 매물
- **Subscription** — 구독·광고 결제 (Phase 2+)
- **Inquiry** — 매물 문의 (Phase 1: 연락처 / Phase 2+: 메시징)

## 의존
- `crates/domain/shared-kernel`
- `crates/domain/core` Aggregate ID만 (cross-BC 직접 호출 금지, 이벤트로 통신)

## 정책
- Core BC와 통신은 도메인 이벤트 + Outbox 패턴
- 실거래가는 immutable (수정 X)
- 경매 데이터는 *법원 공식*만 — 추정·예측 금지
- 결제 처리는 외부 PG (Toss/I'mport, sub-project별 결정)

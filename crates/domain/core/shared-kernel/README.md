# shared-kernel

공짱 도메인 공유 값 객체 (Pnu, Money, Area, BusinessNumber 등) crate예요.

## 책임

- BC 간 공통 어휘 (Pnu, Money, ...)
- *Aggregate*는 각 BC crate에 속해요 (예: `crates/domain/core/listing`)
- DB·HTTP·외부 API 의존 없음 — 순수 값 객체만

## 의존

- `ulid` — domain ID 생성
- `chrono` — 시각 (UTC 저장 + KST 표시)
- `geo-types` — Geometry 좌표
- `serde` — 직렬화 (DB·HTTP 양쪽)
- `thiserror` — 에러 enum
- `regex` + `std::sync::LazyLock` — Email, PhoneKr 검증

## 추가/변경 흐름

1. spec(`docs/superpowers/specs/...`)에 변경 기록
2. 이 crate에 *실패 테스트* 추가
3. 최소 구현 + clippy 통과
4. 다른 crate가 import

## 테스트 커버리지 목표

도메인 로직 ≥ 90% (cargo-tarpaulin로 측정).

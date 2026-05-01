# testing/

테스트 전략·도구·커버리지 SSOT.

## 책임 영역
- 단위 (cargo test, Vitest)
- 통합 (Testcontainers, sqlx::test)
- 계약 (Pact, Phase 3+)
- 스냅샷 (insta, Vitest snapshot)
- Property-based (proptest)
- Mutation (cargo-mutants, 주간 cron)
- E2E (Playwright)
- 부하 (k6)
- 카오스 (Chaos Mesh, Phase 4+)
- 시각 회귀 (Lost Pixel + Storybook)
- 커버리지 임계값 (도메인 90%+)

## 작성 예정 문서 (전반)
- `unit.md` — cargo test + Vitest 패턴
- `integration.md` — Testcontainers + sqlx::test
- `contract-pact.md` — Pact 양쪽 검증
- `property-based.md` — proptest
- `mutation.md` — cargo-mutants 운영
- `e2e.md` — Playwright + data-testid 컨벤션
- `load.md` — k6 시나리오
- `chaos.md` — Chaos Mesh (Phase 4+)
- `visual-regression.md` — Lost Pixel + Chromatic 비교

## 관련 ADR
- (도입 시 ADR 작성)

## 관련 컨벤션
- → @docs/conventions/testing.md

# crates/circuit-breaker

외부 호출 표준 미들웨어 — Circuit Breaker + Retry + Timeout + Idempotency.

## 책임
- Circuit Breaker (failsafe-rs 또는 자체)
- 지수 백오프 + jitter Retry
- Timeout (요청별 설정)
- Idempotency-Key 검증 + 중복 차단
- Audit log (모든 외부 호출)
- Fallback 패턴 (cached response)

## 의존
- `tower` (미들웨어 조합)
- `failsafe`, `governor` (rate limit)
- `crates/observability`

## 정책
- 모든 외부 API 호출은 *반드시* 이 crate 미들웨어 통과
- 각 외부 API별 정책 = `crates/data-clients/<api>/policy.rs`에 명시
- Open 상태 진입 = Sentry alert + Slack 알림
- 직접 reqwest 호출 = lint 차단 (sub-project 5+)

## 정책 예시 (V-World)
- timeout: 10초
- retry: 1회 (1s, 2s 지수)
- circuit open: 5초 내 5회 실패 → 30초 차단
- fallback: cached response (TTL 24h)

→ @docs/backend/circuit-breaker.md, → @docs/data-sources/v-world.md

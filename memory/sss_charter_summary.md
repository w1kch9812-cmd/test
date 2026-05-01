---
name: SSS 7 기둥 요약
description: 이 프로젝트가 만족시켜야 할 SSS급 품질의 7가지 기둥
type: project
---

이 프로젝트(공짱)는 *하이엔드 엔터프라이즈 SSS급*을 목표로 함. SSS의 정의 = 7 기둥 모두 측정 가능하게 만족.

## 7 기둥

1. **일관성 (Consistency)** — 같은 일은 같은 방식으로. 모든 외부 호출 = CB+Retry+Timeout+Audit. 모든 에러 = RFC 9457. 모든 ID = ULID prefix.
2. **자동 강제 (Enforcement)** — 규칙은 사람이 아니라 lefthook + CI가 차단. 시크릿/파일크기/의존성 방향/타입 모두 자동.
3. **추적성 (Traceability)** — 모든 HTTP 요청에 correlation_id, 모든 외부 호출 raw 보존, 모든 결정 ADR, 모든 데이터 변경 audit log.
4. **안전성 (Safety)** — Rust ownership + TS strict + 값 객체(Newtype) + enum exhaustive + Optimistic Locking + Idempotency-Key.
5. **가시성 (Observability)** — OTel + Sentry + Prometheus + Loki + Tempo + Grafana + 합성 모니터링.
6. **SSOT (Single Source of Truth)** — 한 정보 = 한 곳. AWS 콘솔 직접 변경 금지(Pulumi만), TS 타입 수동 작성 금지(OpenAPI 자동).
7. **명확성 (Clarity)** — 컨벤션 9개 (rust/ts/sql/naming/error/ui/test/git/comments) + glossary로 추측 제거.

**Why**: 사용자(2026-05-01)가 "표면적 SSS가 아니라 근본적으로 깔끔한 시스템" 요구. 5 기둥 → 7 기둥 확장(SSOT + Clarity 명시화).
**How to apply**: 매 결정/PR마다 7 기둥 자체 검증. 어느 하나라도 위반하면 그 변경은 미개함.

상세: docs/sss-charter.md (sub-project 1에서 작성)

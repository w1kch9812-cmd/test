---
name: 데이터 접근 규칙 (옵션 A)
description: 메인 시스템은 공식 API 직접만, MCP는 개발자 Claude 세션에서만 사용
type: feedback
---

메인 시스템(`apps/web`)과 개발자 도구(Claude Code 세션)는 **다른 경로**를 사용.

- 메인 시스템: V-World/법제처/공공데이터포털 **공식 REST API 직접**
- 개발자 Claude 세션: korean-land-mcp / korean-law-mcp / opendata-mcp (탐색·학습용)
- 향후 AI 어시스턴트 별도 모듈(옵션): MCP 사용 허용

**Why**: 사용자(2026-04-22 세션)가 프로젝트 범위를 옵션 A(데이터 플랫폼)로 확정. 메인 시스템에 LLM/MCP 의존성을 넣지 않는 것이 SSS급 핵심. AI 기능이 필요해지면 별도 모듈로 격리하기로 결정.

**How to apply**:
- `apps/web/`, `packages/data-clients/` 에는 공식 API HTTP 클라이언트만 구현
- MCP/LLM SDK import가 메인 번들에 들어가지 않도록 린트/훅으로 차단 (계획)
- MCP에서 얻은 결과를 메인 코드에 하드코딩 금지 — 도메인 지식은 reference로 학습 후 직접 작성
- 관리자 도구·일회성 분석 스크립트는 MCP 사용 OK
- 향후 `apps/ai-assistant/` 추가 시 그쪽에서만 MCP 의존성 허용

AGENTS.md §3, scope_option_a.md 참조.

# .agents/

에이전트 공용 자료 (Claude / OpenAI / Cursor / Cline / Aider 등 모든 도구가 공유).

## 정책

- 도구 무관 자료만 (도구별 룰은 `.claude/`, `.cursor/` 등에)
- 모든 AI 도구가 읽을 수 있는 Markdown 형식
- AGENTS.md가 진입점, 이 폴더는 보조 자료

## 향후 추가 (sub-project 단위)

- `subagents/` — 도메인별 subagent 정의 (예: `code-reviewer.md`, `docs-auditor.md`, `compliance-checker.md`)
- `prompts/` — 공용 prompt 템플릿
- `glossary-aliases.md` — 도구별 글로서리 변형 (예: GPT는 *Listing* 인식, 일부 모델은 *Property* 강력 선호 — 이를 *Listing*로 통일)

## SSOT

- 진입점: `AGENTS.md` (루트)
- 도메인 SSOT: `docs/`
- 결정: `docs/adr/`
- 컨벤션: `docs/conventions/`

이 폴더 자체는 *얇음* — 다른 SSOT를 *링크*만.

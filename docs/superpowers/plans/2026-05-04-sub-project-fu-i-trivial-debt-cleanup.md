# Sub-project FU-i: Trivial Debt Cleanup — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.
>
> **CRITICAL pre-read:** [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) + [docs/superpowers/specs/2026-05-04-sub-project-fu-i-trivial-debt-cleanup-design.md](../specs/2026-05-04-sub-project-fu-i-trivial-debt-cleanup-design.md)

**Goal:** 누적 FU 18+ 중 *영역에 안 묶이는* 6 건 (FU 12, 13, 17, 18, 26, 41) 한 번에 청산.

**Architecture:** 각 FU 가 독립적이라 task 묶음 단위 = 영역/파일별. T1 (docs-only), T2 (auth verifier), T3 (clippy.toml), T4 (한글 매핑), T5 (종료 + roadmap 갱신).

**Tech Stack:** Rust 1.88 + clippy + markdownlint. 새 의존성 0.

**환경**: 로컬 cargo 작동 (MSVC). 모든 변경 영향 작아 push 1번 = 1 fix iter 거의 없을 것.

**Repo**: `https://github.com/w1kch9812-cmd/test` (public, Actions free).

---

## Task 분해 (5 task)

- **T1**: FU 12 + 13 + 17 — docs/rustdoc only (1 commit)
- **T2**: FU 18 — auth verifier clippy 빚 정리
- **T3**: FU 26 — workspace `clippy.toml` 에 `disallowed-types reqwest::Client`
- **T4**: FU 41 — `parse_purpose` / `parse_structure` 한글 매핑 확장 + 단위 테스트 ~30
- **T5**: 통합 검증 + `roadmap.md` 갱신 (6 FU ✅ closed 표기)

각 task: 로컬 `cargo check / clippy / test --lib` 통과 후 push → CI 그린 확인.

---

## File Structure

수정:
```
docs/superpowers/specs/2026-05-02-sub-project-2-db-core-domain-design.md       (FU 12 — listing_photo prefix 1줄)
docs/superpowers/specs/2026-05-03-sub-project-5-iii-...-design.md              (FU 13 — audit_log INSERT 컬럼 정정 + § 11 매핑)
crates/domain/audit/audit-log/src/repository.rs                                (FU 17 — find_by_resource/find_by_actor rustdoc)
crates/operations/operations-meta/src/repository.rs                            (FU 17 — find_unacknowledged_alerts rustdoc)
crates/auth/src/verifier.rs                                                    (FU 18 — clippy::panic + manual_let_else)
clippy.toml                                                                    (FU 26 — disallowed-types 추가)
crates/data-clients/data-go-kr/src/building_register/parser.rs                 (FU 41 — parse_purpose + parse_structure 매핑 확장 + 단위 테스트)
docs/superpowers/roadmap.md                                                    (T5 — 6 FU ✅ closed)
```

---

## Plan Parts

Detailed phase bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Docs And Rustdoc Cleanup](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.part-01-docs-rustdoc.md)
- [Part 02 - Auth Clippy And Workspace Lint](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.part-02-auth-clippy-workspace-lint.md)
- [Part 03 - Korean Mapping Expansion](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.part-03-korean-mapping.md)
- [Part 04 - Closeout, Verification, Risks, And Next Steps](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.part-04-closeout-verification.md)

# 코드 컨벤션

> 모든 컨벤션은 *자동 강제*되어야 함. 사람이 지키는 룰은 SSS 미달.

## 학습 순서

1. [naming-and-ids.md](./naming-and-ids.md) — 네이밍 + ULID prefix
2. [rust.md](./rust.md) — Rust (rustfmt + clippy pedantic)
3. [typescript.md](./typescript.md) — TypeScript (Biome v2.4)
4. [sql.md](./sql.md) — PostgreSQL + PostGIS
5. [error-format.md](./error-format.md) — RFC 9457 Problem Details
6. [ui-writing-korean.md](./ui-writing-korean.md) — 사용자 노출 한국어 (해요체)
7. [testing.md](./testing.md) — 테스트 네이밍 + 분류
8. [git-and-pr.md](./git-and-pr.md) — Conventional Commits + PR 룰
9. [comments.md](./comments.md) — Why over What

## 자동 강제 도구 매핑

| 컨벤션 | 도구 | 단계 |
|--------|------|------|
| Rust 포맷/lint | rustfmt + clippy | pre-commit + CI |
| TS 포맷/lint | Biome v2.4 | pre-commit + CI |
| SQL 포맷/lint | sqlfluff (sub-project 2+) | CI |
| Markdown lint | markdownlint-cli2 | pre-commit + CI |
| 커밋 메시지 | commitlint (lefthook) | commit-msg |
| 시크릿 | gitleaks | pre-commit + CI |
| 의존성 방향 | dependency-cruiser + cargo-arch | CI |
| 파일 크기 | 자체 hook | pre-commit + CI |
| 도메인 용어 | grep CI 룰 | CI |

## 위반 처리

- pre-commit 단계 차단 → 즉시 수정 후 재시도
- CI 단계 차단 → PR 머지 불가
- *예외 적용* 필요 시 ADR 작성 + 팀 승인 (Phase 2+)

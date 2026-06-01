# Sub-project 1 - Part 01A: Overview, File Structure, Environment, And Git Setup

Parent index: [Sub-project 1 Part 01](./2026-05-01-sub-project-1-charter-monorepo.part-01.md).
# Sub-project 1: 프로젝트 헌법 + 모노레포 셋업 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** [docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md](../specs/2026-05-01-sub-project-1-charter-monorepo-design.md)에 정의된 SSS급 엔터프라이즈 모노레포 기반 (60-80개 파일)을 의존 그래프 순서로 구축한다. 코드 0줄, 문서·설정·CI 워크플로우만.

**Architecture:** 17개 Task로 분해. 의존 흐름은 *모노레포 골격 → 자동 강제 도구 → CI → 진입점 문서 → SSS SSOT → ADR/컨벤션/도메인 README → 워크스페이스 멤버 README → GitHub 메타 → 최종 검증 + PR*. 각 Task는 *파일 작성 → 자동 검증(Biome/sgrep/markdown-link-check) → 커밋* 흐름.

**Tech Stack:** pnpm 9.12+, Cargo 1.83+, Turborepo 2, TypeScript 5.7, Biome 2.4, lefthook, gitleaks, GitHub Actions, markdown-link-check, Zitadel(향후), sqlfluff, dependency-cruiser.

**SSS 7 기둥 (이 plan이 만족시켜야 할 기준):**
1. 일관성 — 모든 파일 같은 형식, 같은 헤더, 같은 footer
2. 자동 강제 — Task 4-6에서 lefthook + CI로 강제 시작
3. 추적성 — Task 10에서 ADR 11개로 모든 결정 영구 기록
4. 안전성 — TS strict + Rust strict + Biome strict 처음부터
5. 가시성 — Task 16에서 OTel/Sentry 자리 명시 (구현은 sub-project 7)
6. SSOT — Task 8에서 ssot-matrix.md 작성, 위반 자동 검출 룰 정의
7. 명확성 — Task 11에서 9개 컨벤션 문서

---

## File Structure

총 76개 파일. 카테고리별 책임.

### 루트 진입점 (10개)
- `README.md` — 사람용 프로젝트 소개 (≤200줄)
- `AGENTS.md` — 오픈 표준 라우터 (≤300줄)
- `CLAUDE.md` — `Read @AGENTS.md` 1줄
- `TECH.md` — 기술 스택 + SSOT 맵 (≤300줄)
- `MEMORY.md` — 자동 메모리 인덱스
- `LICENSE` — 한 줄 (`Copyright © 2026 공짱. All Rights Reserved.`)
- `.gitignore`
- `.editorconfig`
- `.gitattributes`
- `.nvmrc` — Node 20.18

### 모노레포 설정 (10개)
- `Cargo.toml` — Rust workspace (members + 공유 deps + 공유 lints)
- `rust-toolchain.toml` — Rust 1.83 stable
- `clippy.toml` — clippy pedantic 룰
- `rustfmt.toml` — Rust 포맷
- `deny.toml` — cargo-deny (라이선스 + 보안)
- `package.json` — pnpm root + scripts
- `pnpm-workspace.yaml` — workspace members
- `turbo.json` — Turborepo pipelines
- `tsconfig.base.json` — TS strict 기본
- `.env.example` — 환경 변수 placeholder

### 자동 강제 (6개)
- `biome.json` — Biome 2.4 (format + lint + import sort)
- `lefthook.yml` — pre-commit + pre-push hooks
- `.gitleaks.toml` — 시크릿 스캔 룰
- `markdownlint.json` — Markdown lint 룰
- `renovate.json` — 의존성 자동 업데이트 룰
- `.github/workflows/ci.yml` — 메인 CI 파이프라인

### GitHub 메타 (4개)
- `.github/CODEOWNERS`
- `.github/pull_request_template.md`
- `.github/ISSUE_TEMPLATE/bug.md`
- `.github/ISSUE_TEMPLATE/feature.md`

### .claude / .agents / .mcp (3개)
- `.claude/settings.json` — Claude Code 설정
- `.mcp.json` — MCP 서버 (Zitadel docs MCP 자리, 향후)
- `.agents/README.md` — 에이전트 공용 자리 표시

### docs/ 진입점 (4개)
- `docs/README.md` — 도메인 카테고리 목차
- `docs/sss-charter.md` — 7 기둥 + 15 검증 SSOT
- `docs/glossary.md` — 한·영 도메인 용어 사전
- `docs/ssot-matrix.md` — 정보별 SSOT + 위반 자동 차단 룰

### docs/adr/ (12개: README + 11)
- `docs/adr/README.md` — ADR 인덱스 + 작성 가이드 + 템플릿
- `docs/adr/0001-language-rust-ts.md`
- `docs/adr/0002-monorepo-cargo-pnpm-turbo.md`
- `docs/adr/0003-frontend-nextjs-react19.md`
- `docs/adr/0004-db-postgres-postgis.md`
- `docs/adr/0005-auth-zitadel.md`
- `docs/adr/0006-api-rest-openapi.md`
- `docs/adr/0007-cache-moka-valkey.md`
- `docs/adr/0008-observability-grafana-otel-sentry.md`
- `docs/adr/0009-iac-pulumi.md`
- `docs/adr/0010-scope-information-platform-option-a.md`
- `docs/adr/0011-embedding-gemini-pgvector.md`

### docs/conventions/ (10개: README + 9)
- `docs/conventions/README.md`
- `docs/conventions/rust.md`
- `docs/conventions/typescript.md`
- `docs/conventions/sql.md`
- `docs/conventions/naming-and-ids.md`
- `docs/conventions/error-format.md`
- `docs/conventions/ui-writing-korean.md`
- `docs/conventions/testing.md`
- `docs/conventions/git-and-pr.md`
- `docs/conventions/comments.md`

### docs/{도메인 카테고리}/ (13개)
- `docs/infrastructure/README.md`
- `docs/auth/README.md`
- `docs/data/README.md`
- `docs/cache-messaging/README.md`
- `docs/backend/README.md`
- `docs/api/README.md`
- `docs/observability/README.md`
- `docs/security/README.md`
- `docs/testing/README.md`
- `docs/frontend/README.md`
- `docs/governance/README.md`
- `docs/compliance/README.md`
- `docs/cost/README.md`

### docs/data-sources/ (6개)
- `docs/data-sources/README.md`
- `docs/data-sources/v-world.md`
- `docs/data-sources/data-go-kr.md`
- `docs/data-sources/korean-law.md`
- `docs/data-sources/nice-identity.md`
- `docs/data-sources/naver-maps.md`

### 워크스페이스 멤버 README (28개 — apps/services/crates/packages 등)
- `apps/platform-web/README.md`
- `apps/admin-web/README.md`
- `services/api/README.md`
- `services/worker/README.md`
- `services/data-pipeline/README.md`
- `crates/domain/core/README.md`
- `crates/domain/market/README.md`
- `crates/domain/regulation/README.md`
- `crates/domain/insights/README.md`
- `crates/domain/shared-kernel/README.md`
- `crates/data-clients/README.md`
- `crates/db/README.md`
- `crates/geo/README.md`
- `crates/auth/README.md`
- `crates/cache/README.md`
- `crates/observability/README.md`
- `crates/circuit-breaker/README.md`
- `crates/api-types/README.md`
- `crates/audit/README.md`
- `crates/embedding/README.md`
- `packages/ui-web/README.md`
- `packages/api-client/README.md`
- `packages/shared/README.md`
- `packages/map/README.md`
- `packages/tsconfig/README.md`
- `infrastructure/README.md`
- `tools/README.md`
- `db/migration/README.md`

**총 합산: 10 + 10 + 6 + 4 + 3 + 4 + 12 + 10 + 13 + 6 + 28 = 106개**

(추정 60-80에서 늘어남. README 스켈레톤이 워크스페이스 멤버 28개 추가로 계산되어서. 실제 작업량은 비슷 — 워크스페이스 README는 매우 짧음)

---

## Tasks

### Task 1: 환경 검증 + 기존 폴더 정리

**Files:**
- 정리: 기존 `apps/`, `packages/`, `crates/`, `services/` 등 빈 폴더 (없으면 skip)
- 생성: `_archive/2026-05-01-pre-charter/` (있다면 백업)

- [ ] **Step 1: Rust 1.83+ / Node 20+ / pnpm 9.12+ 확인**

```bash
rustc --version    # rustc 1.83.0 이상 기대
node --version     # v20.18.0 이상 기대
pnpm --version     # 9.12.0 이상 기대
git --version      # 2.40 이상
```

설치 안 되어 있으면:
- Rust: https://rustup.rs (Windows: rustup-init.exe)
- Node: nvm으로 v20.18 설치 (`nvm install 20`)
- pnpm: `corepack enable && corepack prepare pnpm@9.12.0 --activate`

- [ ] **Step 2: 현재 폴더 상태 확인**

```bash
cd /c/Users/User/Desktop/gongzzang_2
ls -la
```

기존 셋업 잔재 (apps/, packages/, crates/, services/, infrastructure/, tools/, reference/, db/, tests/) 가 있다면 백업.

- [ ] **Step 3: 기존 잔재 백업 (있는 경우만)**

```bash
mkdir -p _archive/2026-05-01-pre-charter
for dir in apps packages crates services infrastructure tools reference db tests; do
  if [ -d "$dir" ]; then
    mv "$dir" "_archive/2026-05-01-pre-charter/"
  fi
done
ls -la _archive/2026-05-01-pre-charter/
```

빈 디렉토리는 보존할 필요 없으니 그냥 삭제도 OK:
```bash
for dir in apps packages crates services infrastructure tools reference db tests; do
  [ -d "$dir" ] && rmdir "$dir" 2>/dev/null || rm -rf "$dir"
done
```

- [ ] **Step 4: 검증**

```bash
ls -la
```
기대: 기존 잔재 디렉토리 없음. docs/, memory/, .claude/, .gitignore, AGENTS.md (이전 버전), CLAUDE.md, README.md, TECH.md, MEMORY.md, .mcp.json은 일단 그대로 (Task 6에서 갱신).

- [ ] **Step 5: 커밋 (선택)**

```bash
git add _archive/
git commit -m "chore: archive pre-charter scaffolding before sub-project 1"
```

git 미초기화 상태면 Task 2에서 한꺼번에.

---

### Task 2: Git 초기화 + GitHub 레포 연결

**Files:**
- 사용: 기존 `.gitignore` (Task 5에서 갱신)
- 신규: 없음

- [ ] **Step 1: Git 상태 확인**

```bash
git status 2>&1 | head -3
```

"not a git repository"면 초기화 필요. 이미 git repo면 다음 단계로.

- [ ] **Step 2: Git 초기화 (필요 시)**

```bash
git init
git branch -M main
```

- [ ] **Step 3: GitHub 원격 연결**

```bash
git remote add origin https://github.com/w1kch9812-cmd/gongzzang3.git
git remote -v
```

기대 출력:
```
origin  https://github.com/w1kch9812-cmd/gongzzang3.git (fetch)
origin  https://github.com/w1kch9812-cmd/gongzzang3.git (push)
```

이미 origin이 있으면:
```bash
git remote set-url origin https://github.com/w1kch9812-cmd/gongzzang3.git
```

- [ ] **Step 4: 인증 확인 (Personal Access Token 또는 SSH)**

HTTPS면 PAT 필요. 또는 SSH 키 셋업:
```bash
git config --global user.name "<your-name>"
git config --global user.email "<your-email>"
```

- [ ] **Step 5: 빈 첫 커밋 (Foundation Marker)**

```bash
git commit --allow-empty -m "chore: initialize sub-project 1 (charter + monorepo)"
git push -u origin main
```

push 권한 OK 확인. 첫 push가 실패하면 PAT 또는 SSH 키 셋업부터.

---

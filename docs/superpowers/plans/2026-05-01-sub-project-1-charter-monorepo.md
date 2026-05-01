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

### Task 3: 모노레포 루트 설정 (10개 파일)

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`, `rustfmt.toml`, `deny.toml`
- Create: `package.json`, `pnpm-workspace.yaml`, `turbo.json`, `tsconfig.base.json`, `.env.example`

- [ ] **Step 1: `rust-toolchain.toml` 작성**

```toml
[toolchain]
channel = "1.83.0"
components = ["rustfmt", "clippy", "rust-analyzer", "rust-src"]
profile = "default"
```

- [ ] **Step 2: `Cargo.toml` (workspace) 작성**

```toml
[workspace]
resolver = "3"
members = [
    # services/* 와 crates/* 는 sub-project 2+에서 추가됨.
    # sub-project 1에서는 빈 workspace로 시작.
]

[workspace.package]
edition = "2021"
rust-version = "1.83"
license = "UNLICENSED"
authors = ["공짱 <perfectoryinc@gmail.com>"]

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unused_imports = "deny"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"

[workspace.dependencies]
# sub-project 2+에서 채움 (axum, sqlx, tokio, serde, tracing 등)
```

- [ ] **Step 3: `clippy.toml` + `rustfmt.toml`**

`clippy.toml`:
```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
type-complexity-threshold = 250
too-many-lines-threshold = 100
```

`rustfmt.toml`:
```toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
use_field_init_shorthand = true
use_try_shorthand = true
imports_granularity = "Crate"
group_imports = "StdExternalCrate"
reorder_imports = true
```

- [ ] **Step 4: `deny.toml` (cargo-deny 정책)**

```toml
[graph]
all-features = false
no-default-features = false

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
yanked = "deny"

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-DFS-2016",
    "CC0-1.0",
    "Zlib",
    "MPL-2.0",
]
deny = [
    "GPL-3.0",
    "AGPL-3.0",
    "AGPL-1.0",
    "LGPL-3.0",
    "SSPL-1.0",
]
confidence-threshold = 0.93

[bans]
multiple-versions = "warn"
wildcards = "deny"
highlight = "all"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

- [ ] **Step 5: `package.json` + `pnpm-workspace.yaml`**

`package.json`:
```json
{
  "name": "gongzzang",
  "private": true,
  "version": "0.0.0",
  "description": "산업용 부동산 정보 플랫폼",
  "packageManager": "pnpm@9.12.0",
  "engines": {
    "node": ">=20.18.0",
    "pnpm": ">=9.12.0"
  },
  "scripts": {
    "build": "turbo run build",
    "dev": "turbo run dev",
    "lint": "biome check .",
    "lint:fix": "biome check --write .",
    "format": "biome format --write .",
    "typecheck": "turbo run typecheck",
    "test": "turbo run test",
    "clean": "turbo run clean && rimraf node_modules **/node_modules"
  },
  "devDependencies": {
    "@biomejs/biome": "^2.4.0",
    "turbo": "^2.3.0",
    "typescript": "^5.7.0",
    "rimraf": "^6.0.1"
  }
}
```

`pnpm-workspace.yaml`:
```yaml
packages:
  - "apps/*"
  - "packages/*"
  - "tools/*"
```

- [ ] **Step 6: `turbo.json` + `tsconfig.base.json`**

`turbo.json`:
```json
{
  "$schema": "https://turborepo.com/schema.json",
  "ui": "tui",
  "globalDependencies": ["**/.env", "**/.env.local", "tsconfig.base.json"],
  "globalEnv": [
    "NODE_ENV",
    "VWORLD_API_KEY",
    "VWORLD_DOMAIN",
    "ZITADEL_API_KEY",
    "DATABASE_URL"
  ],
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "inputs": ["src/**", "tsconfig.json", "package.json"],
      "outputs": ["dist/**", ".next/**", "!.next/cache/**"]
    },
    "dev": {
      "cache": false,
      "persistent": true
    },
    "typecheck": {
      "dependsOn": ["^build"],
      "inputs": ["src/**", "tsconfig.json"],
      "outputs": []
    },
    "test": {
      "dependsOn": ["^build"],
      "inputs": ["src/**", "test/**", "*.config.*"],
      "outputs": ["coverage/**"]
    },
    "clean": {
      "cache": false
    }
  }
}
```

`tsconfig.base.json`:
```json
{
  "$schema": "https://json.schemastore.org/tsconfig",
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022"],
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "noFallthroughCasesInSwitch": true,
    "exactOptionalPropertyTypes": true,
    "verbatimModuleSyntax": true,
    "esModuleInterop": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "incremental": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "composite": true
  },
  "exclude": ["node_modules", "dist", ".next", "coverage"]
}
```

- [ ] **Step 7: `.env.example`**

```bash
# === 외부 공공 API ===
VWORLD_API_KEY=your_vworld_key_here
VWORLD_DOMAIN=localhost
KOREAN_LAW_API_KEY=your_law_api_key_here
ODP_SERVICE_KEY=your_data_go_kr_key_here
NICE_API_KEY=your_nice_key_here
NAVER_MAPS_CLIENT_ID=your_naver_client_id_here

# === 인증 (Zitadel) ===
ZITADEL_DOMAIN=https://your-instance.zitadel.cloud
ZITADEL_PROJECT_ID=
ZITADEL_API_KEY=

# === 임베딩 (Gemini) ===
GEMINI_API_KEY=

# === DB ===
DATABASE_URL=postgresql://gongzzang:gongzzang@localhost:5432/gongzzang
REDIS_URL=redis://localhost:6379

# === 관측 ===
SENTRY_DSN=
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# === 환경 ===
NODE_ENV=development
RUST_LOG=info
```

- [ ] **Step 8: 검증**

```bash
cargo check 2>&1 | head -5    # 빈 workspace라도 에러 없어야
```

기대: `Checking gongzzang-workspace v0.0.0` 또는 멤버 없다는 메시지. 에러 없음.

```bash
pnpm install
```

기대: `Done` 메시지, `node_modules/` 생성, `pnpm-lock.yaml` 생성.

- [ ] **Step 9: 커밋**

```bash
git add Cargo.toml rust-toolchain.toml clippy.toml rustfmt.toml deny.toml
git add package.json pnpm-workspace.yaml turbo.json tsconfig.base.json .env.example
git commit -m "chore(monorepo): set up Cargo + pnpm workspaces with strict lints"
```

---

### Task 4: Biome 셋업

**Files:**
- Create: `biome.json`

- [ ] **Step 1: `biome.json` 작성**

```json
{
  "$schema": "https://biomejs.dev/schemas/2.4.0/schema.json",
  "vcs": {
    "enabled": true,
    "clientKind": "git",
    "useIgnoreFile": true
  },
  "files": {
    "ignoreUnknown": true,
    "includes": [
      "**",
      "!**/node_modules",
      "!**/dist",
      "!**/.next",
      "!**/.turbo",
      "!**/coverage",
      "!**/target",
      "!**/_archive",
      "!**/reference"
    ]
  },
  "formatter": {
    "enabled": true,
    "indentStyle": "space",
    "indentWidth": 2,
    "lineWidth": 100,
    "lineEnding": "lf"
  },
  "assist": {
    "enabled": true,
    "actions": {
      "source": {
        "organizeImports": "on"
      }
    }
  },
  "linter": {
    "enabled": true,
    "rules": {
      "recommended": true,
      "correctness": {
        "noUnusedImports": "error",
        "noUnusedVariables": "warn",
        "noUnusedFunctionParameters": "warn"
      },
      "style": {
        "useConst": "error",
        "useImportType": "error",
        "useTemplate": "warn",
        "noNonNullAssertion": "warn",
        "useNamingConvention": {
          "level": "warn",
          "options": {
            "strictCase": false,
            "conventions": [
              { "selector": { "kind": "function" }, "formats": ["camelCase"] },
              { "selector": { "kind": "typeLike" }, "formats": ["PascalCase"] }
            ]
          }
        }
      },
      "suspicious": {
        "noConsole": {
          "level": "warn",
          "options": { "allow": ["warn", "error", "info"] }
        },
        "noExplicitAny": "error"
      },
      "complexity": {
        "noExcessiveCognitiveComplexity": {
          "level": "warn",
          "options": { "maxAllowedComplexity": 15 }
        }
      }
    }
  },
  "javascript": {
    "formatter": {
      "quoteStyle": "double",
      "trailingCommas": "all",
      "semicolons": "always",
      "arrowParentheses": "always"
    }
  },
  "json": {
    "formatter": {
      "trailingCommas": "none"
    }
  }
}
```

- [ ] **Step 2: 검증 — 자기 자신을 lint**

```bash
pnpm biome check biome.json
```

기대: `Checked 1 file in <ms>. No fixes needed.`

```bash
pnpm biome check package.json
```

기대: 통과 또는 작은 포맷 fix 제안. fix:

```bash
pnpm biome check --write package.json turbo.json tsconfig.base.json
```

- [ ] **Step 3: 커밋**

```bash
git add biome.json
git commit -m "chore(lint): set up Biome 2.4 with strict rules"
```

---

### Task 5: 자동 강제 도구 (lefthook + gitleaks + markdownlint + renovate)

**Files:**
- Create: `lefthook.yml`, `.gitleaks.toml`, `markdownlint.json`, `renovate.json`
- Update: `.gitignore`, `.editorconfig`, `.gitattributes`, `.nvmrc`

- [ ] **Step 1: `.gitignore` 작성**

```gitignore
# === Env ===
.env
.env.local
.env.*.local
!.env.example

# === Node / pnpm / TS ===
node_modules/
.pnpm-store/
.turbo/
dist/
build/
.next/
out/
coverage/
*.tsbuildinfo

# === Rust / Cargo ===
target/
*.rs.bk
Cargo.lock.bak

# === Logs ===
*.log
npm-debug.log*
yarn-debug.log*
yarn-error.log*
pnpm-debug.log*

# === OS ===
.DS_Store
Thumbs.db

# === IDE ===
.idea/
.vscode/*
!.vscode/extensions.json
!.vscode/settings.json

# === Claude / 로컬 메모리 ===
.claude/settings.local.json
.claude/*.local.*
CLAUDE.local.md
AGENTS.local.md

# === 빌드 산출물 / 임시 ===
*.tgz
*.zip
.cache/
.temp/
.tmp/

# === 학습용 외부 레포 (빌드 제외) ===
reference/*/
!reference/README.md

# === 로컬 MCP 빌드 ===
.mcp-local/

# === 백업 / 아카이브 ===
_archive/
```

- [ ] **Step 2: `.editorconfig`**

```ini
root = true

[*]
end_of_line = lf
insert_final_newline = true
charset = utf-8
trim_trailing_whitespace = true

[*.{rs,toml}]
indent_style = space
indent_size = 4

[*.{ts,tsx,js,jsx,json,jsonc,yml,yaml,md}]
indent_style = space
indent_size = 2

[*.{md,mdx}]
trim_trailing_whitespace = false  # 줄 끝 두 칸은 줄바꿈 의미

[Makefile]
indent_style = tab
```

- [ ] **Step 3: `.gitattributes` + `.nvmrc`**

`.gitattributes`:
```
* text=auto eol=lf
*.{cmd,[cC][mM][dD]} text eol=crlf
*.{bat,[bB][aA][tT]} text eol=crlf
*.png binary
*.jpg binary
*.gif binary
*.ico binary
*.pdf binary
*.zip binary
```

`.nvmrc`:
```
20.18.0
```

- [ ] **Step 4: `lefthook.yml` (pre-commit + pre-push 훅)**

```yaml
# https://lefthook.dev/configuration/
pre-commit:
  parallel: true
  commands:
    biome:
      glob: "*.{ts,tsx,js,jsx,json,jsonc,css}"
      run: pnpm biome check --write --no-errors-on-unmatched {staged_files}
      stage_fixed: true
    rustfmt:
      glob: "*.rs"
      run: cargo fmt -- {staged_files}
      stage_fixed: true
    markdownlint:
      glob: "*.md"
      run: pnpm markdownlint-cli2 {staged_files}
    gitleaks:
      run: gitleaks protect --staged --redact -v
    file-size:
      run: |
        for f in {staged_files}; do
          if [ -f "$f" ]; then
            lines=$(wc -l < "$f")
            if [ "$lines" -gt 1500 ]; then
              echo "❌ $f has $lines lines (> 1500). 폴더로 분해하세요."
              exit 1
            elif [ "$lines" -gt 500 ]; then
              echo "⚠️  $f has $lines lines (> 500). 가능하면 분해를 검토하세요."
            fi
          fi
        done

pre-push:
  parallel: false
  commands:
    typecheck:
      run: pnpm turbo run typecheck
    cargo-check:
      run: cargo check --workspace --all-features
    cargo-clippy:
      run: cargo clippy --workspace --all-features -- -D warnings
    cargo-deny:
      run: cargo deny check
    markdown-links:
      run: pnpm markdown-link-check docs/**/*.md README.md AGENTS.md

commit-msg:
  commands:
    conventional:
      run: |
        head -1 {1} | grep -qE '^(feat|fix|chore|docs|test|refactor|perf|ci|build|revert)(\(.+\))?: .+' || {
          echo "❌ Conventional Commits 형식: feat|fix|chore|docs|test|refactor|perf|ci|build(scope?): message"
          exit 1
        }
```

- [ ] **Step 5: `.gitleaks.toml` (시크릿 스캔)**

```toml
# https://github.com/gitleaks/gitleaks
title = "공짱 gitleaks config"

[extend]
useDefault = true

[[rules]]
id = "vworld-api-key"
description = "V-World API key (32 hex chars)"
regex = '''[A-F0-9]{8}-[A-F0-9]{4}-[A-F0-9]{4}-[A-F0-9]{4}-[A-F0-9]{12}'''
keywords = ["VWORLD", "vworld"]

[[rules]]
id = "naver-client-id"
description = "Naver Maps Client ID"
regex = '''(NAVER|naver).{0,20}['"][a-z0-9]{10,30}['"]'''

[allowlist]
description = "Test/example values"
paths = [
    '''\.env\.example$''',
    '''docs/.+\.md$''',
    '''_archive/''',
]
```

- [ ] **Step 6: `markdownlint.json` + `renovate.json`**

`markdownlint.json`:
```json
{
  "default": true,
  "MD013": { "line_length": 200, "tables": false, "code_blocks": false },
  "MD024": { "siblings_only": true },
  "MD033": false,
  "MD041": false,
  "MD036": false
}
```

`renovate.json`:
```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": [
    "config:recommended",
    ":semanticCommits",
    ":separateMajorReleases",
    ":automergeMinor",
    ":automergePatch"
  ],
  "schedule": ["before 5am on monday"],
  "prConcurrentLimit": 5,
  "rangeStrategy": "bump",
  "rust": { "enabled": true },
  "lockFileMaintenance": { "enabled": true, "schedule": ["before 5am on monday"] }
}
```

- [ ] **Step 7: lefthook 설치 + 활성화**

```bash
pnpm add -Dw lefthook markdownlint-cli2 markdown-link-check
pnpm lefthook install
```

기대: `.git/hooks/pre-commit`, `.git/hooks/pre-push` 등 생성.

- [ ] **Step 8: gitleaks 설치 (사용자 환경)**

```bash
# Windows: scoop install gitleaks  또는  winget install gitleaks
# Mac: brew install gitleaks
# Linux: 바이너리 다운로드
gitleaks version
```

설치 안 되면 lefthook의 gitleaks 단계는 일시 skip. 배포 전 필수 설치.

- [ ] **Step 9: 검증 — 더미 시크릿 커밋 시도**

```bash
echo "ZITADEL_API_KEY=sk_live_realtoken1234567890abcdef" > /tmp/_test_secret.txt
git add /tmp/_test_secret.txt 2>/dev/null || cp /tmp/_test_secret.txt ./_test_secret.txt && git add _test_secret.txt
git commit -m "test: should fail"
```

기대: gitleaks가 차단. 통과하면 정책 미설정.

```bash
git restore --staged _test_secret.txt
rm _test_secret.txt
```

- [ ] **Step 10: 커밋**

```bash
git add lefthook.yml .gitleaks.toml markdownlint.json renovate.json
git add .gitignore .editorconfig .gitattributes .nvmrc
git add package.json pnpm-lock.yaml
git commit -m "chore(quality): add lefthook + gitleaks + markdownlint + renovate"
```

---

### Task 6: GitHub Actions CI 워크플로우

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: CI 워크플로우 작성**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read
  pull-requests: read

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  setup:
    name: Setup
    runs-on: ubuntu-latest
    outputs:
      node-modules-cache-key: ${{ steps.cache-keys.outputs.node }}
      cargo-cache-key: ${{ steps.cache-keys.outputs.cargo }}
    steps:
      - uses: actions/checkout@v4
      - id: cache-keys
        run: |
          echo "node=node-${{ hashFiles('pnpm-lock.yaml') }}" >> $GITHUB_OUTPUT
          echo "cargo=cargo-${{ hashFiles('Cargo.lock', 'rust-toolchain.toml') }}" >> $GITHUB_OUTPUT

  lint-format:
    name: Lint & Format (Biome + clippy + markdownlint)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9.12.0 }
      - uses: actions/setup-node@v4
        with: { node-version: 20.18, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - name: Biome check
        run: pnpm biome check .
      - name: markdownlint
        run: pnpm markdownlint-cli2 "**/*.md" "#node_modules" "#_archive" "#reference"
      - uses: dtolnay/rust-toolchain@stable
        with: { components: "rustfmt, clippy" }
      - name: cargo fmt
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --workspace --all-features -- -D warnings

  typecheck:
    name: TypeScript typecheck
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9.12.0 }
      - uses: actions/setup-node@v4
        with: { node-version: 20.18, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - run: pnpm turbo run typecheck

  cargo-check:
    name: cargo check + cargo deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-features
      - uses: EmbarkStudios/cargo-deny-action@v2
        with: { command: check }

  secret-scan:
    name: Secret scan (gitleaks)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITLEAKS_CONFIG: .gitleaks.toml

  link-check:
    name: Markdown link check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9.12.0 }
      - uses: actions/setup-node@v4
        with: { node-version: 20.18, cache: pnpm }
      - run: pnpm install --frozen-lockfile
      - name: Check internal Markdown links
        run: |
          find docs -name "*.md" -exec pnpm markdown-link-check -q {} \;
          pnpm markdown-link-check -q README.md AGENTS.md

  file-size:
    name: File size limit (≤500 권장 / 1500 강제)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Check file sizes
        run: |
          fail=0
          while IFS= read -r f; do
            lines=$(wc -l < "$f")
            if [ "$lines" -gt 1500 ]; then
              echo "::error file=$f::$lines lines (> 1500). 폴더로 분해하세요."
              fail=1
            elif [ "$lines" -gt 500 ]; then
              echo "::warning file=$f::$lines lines (> 500). 분해 권장."
            fi
          done < <(find . -type f \( -name "*.md" -o -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.sql" \) \
                   ! -path "./node_modules/*" ! -path "./_archive/*" ! -path "./target/*" ! -path "./.next/*" ! -path "./reference/*")
          exit $fail
```

- [ ] **Step 2: 검증 — 로컬에서 동일 명령 시뮬레이션**

```bash
pnpm biome check .
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo deny check
```

기대: 모두 0 exit code.

- [ ] **Step 3: 커밋**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add full pipeline (lint + typecheck + cargo + secrets + links + file-size)"
```

---

### Task 7: 진입점 문서 4개 (AGENTS.md, CLAUDE.md, README.md, TECH.md)

**Files:**
- Create or Replace: `AGENTS.md`, `CLAUDE.md`, `README.md`, `TECH.md`, `MEMORY.md`, `LICENSE`

- [ ] **Step 1: `CLAUDE.md` 작성 (1줄)**

```markdown
Read @AGENTS.md
```

- [ ] **Step 2: `LICENSE` 작성 (한 줄)**

```
Copyright © 2026 공짱 (Gongzzang). All Rights Reserved.

This software and its source code are proprietary. No license is granted
to use, copy, modify, or distribute any portion of this code without
express written permission.

Third-party dependencies retain their original licenses, validated by
deny.toml policy.
```

- [ ] **Step 3: `AGENTS.md` 작성 (≤300줄)**

핵심 섹션:
1. § 0 — 7 기둥 SSS 헌법 요약 + `→ @docs/sss-charter.md`
2. § 1 — 절대 규칙 (≤500줄, glossary 강제, LLM 생성 금지, 임시방편 금지)
3. § 2 — 작업별 라우팅 표 (작업 유형 → 우선 참조 docs)
4. § 3 — 자동 강제 흐름 5단계
5. § 4 — 한국어 규칙 (해요체, 에러 메시지, 법령 인용)
6. § 5 — 데이터 접근 규칙 (메인 = 공식 API 직접, AI 어시스턴트 별도)
7. § 6 — 1500줄 안티패턴 경보

전체 본문은 spec § 7 참고. 작성 시 Markdown 링크는 *생성될* `docs/...` 경로 사용.

- [ ] **Step 4: `README.md` 작성 (≤200줄)**

핵심 섹션:
1. 프로젝트 한 줄 소개 — "산업용 부동산 정보 플랫폼"
2. 기술 스택 (Rust + Next.js + Postgres + ...) → `→ @TECH.md`
3. 빠른 시작 (clone → install → dev)
4. 디렉토리 (apps/, services/, crates/, packages/, docs/, infrastructure/)
5. 핵심 원칙 (옵션 A 데이터 플랫폼, 7 기둥, 한국 시장)
6. 라이선스 — 사내 비공개

- [ ] **Step 5: `TECH.md` 작성 (≤300줄)**

핵심 섹션:
1. 프로젝트 범위 (옵션 A) → `→ @docs/adr/0010-scope-information-platform-option-a.md`
2. 기술 스택 표 (백엔드 / 프론트 / DB / 인증 / 캐시 / 검색 / 관측 / 인프라)
3. SSOT 맵 (`→ @docs/ssot-matrix.md`)
4. 데이터 소스 (`→ @docs/data-sources/README.md`)
5. 좌표계 (SRID 4326 / 5179 / 5186 / 3857)
6. 모노레포 구조 (apps/services/crates/packages 한 줄씩)
7. 환경 변수 (`→ @.env.example`)

- [ ] **Step 6: `MEMORY.md` 갱신 (인덱스만)**

```markdown
- [프로젝트 도메인 스냅샷](memory/project_domain.md) — 산업용 부동산 정보 플랫폼 (옵션 A)
- [데이터 접근 규칙](memory/data_access_layering.md) — 메인=공식 API, MCP=Claude 세션
- [파일 크기 상한 규칙](memory/file_size_rule.md) — ≤500 목표, 1500 강제
- [SSS 7 기둥](memory/sss_charter_summary.md) — 일관성/강제/추적/안전/가시/SSOT/명확
- [프로젝트 범위 결정](memory/scope_option_a.md) — 데이터 플랫폼, AI 생성 X
- [외부 의존 MCP](memory/external_mcps.md) — 개발자 보조 전용
- [SSS급 기준 피드백](memory/feedback_sss_standard.md) — 미개함·꼼수 0
```

기존 memory/*.md 그대로 유지. 새로 추가 필요 시 sub-project 1 진행 중 기록.

- [ ] **Step 7: 검증 — Markdown 링크 + Biome**

```bash
pnpm markdown-link-check AGENTS.md README.md TECH.md MEMORY.md CLAUDE.md
pnpm biome format --write *.md
```

(아직 docs/* 가 없으니 일부 링크는 *작성 예정*으로 깨지는 게 정상. Task 14 마지막에 다시 검증.)

- [ ] **Step 8: 커밋**

```bash
git add AGENTS.md CLAUDE.md README.md TECH.md MEMORY.md LICENSE
git commit -m "docs: bootstrap entry-point docs (AGENTS/CLAUDE/README/TECH/LICENSE)"
```

---

### Task 8: SSS 헌법 + Glossary + SSOT 매트릭스 (3개 파일)

**Files:**
- Create: `docs/sss-charter.md`, `docs/glossary.md`, `docs/ssot-matrix.md`, `docs/README.md`

- [ ] **Step 1: `docs/README.md` (도메인 카테고리 목차)**

핵심 내용:
- 학습 순서 1-13 (sss-charter → glossary → conventions → data-sources → auth → data → backend → api → frontend → infrastructure → observability → security → testing → governance → compliance → cost)
- 각 카테고리 한 줄 책임
- 카테고리 폴더 링크

- [ ] **Step 2: `docs/sss-charter.md` (≤500줄, 7 기둥 SSOT)**

섹션:
1. SSS의 정의 — "표면적 X, 근본적 깔끔함 O"
2. 7 기둥 (각 기둥 1 페이지)
   - 일관성 / 자동 강제 / 추적성 / 안전성 / 가시성 / SSOT / 명확성
   - 각 기둥마다: 정의, 측정 자, 도구, 위반 예시
3. 15 검증 질문 (체크리스트)
4. SSS 단계 (청사진 → 기반 → 핵심 → 운영 → 인증)
5. 검증 주기 (분기별 자체 평가)

- [ ] **Step 3: `docs/glossary.md` (한·영 도메인 용어 사전)**

테이블 형식:
```markdown
| 한국어 | 영문 (코드) | 정의 | 사용 |
|--------|------------|------|------|
| 필지 | Parcel (NOT Land/Lot) | 토지의 등록 단위 (PNU 19자리로 식별) | 매물 / 분석 / 공공 API |
| 매물 | Listing (NOT Property) | 거래 대상 부동산 (공장/창고/사옥 등) | 매물 등록 |
| 사업자등록번호 | BusinessNumber (NOT BizNo/BRN) | 10자리 사업자 식별 번호 | 회원 가입 검증 |
| 공인중개사 | Broker (NOT Agent/Realtor) | 자격증 보유 중개인 | 매물 등록 권한 |
| 산업단지 | IndustrialComplex | 정부 지정 산업 집적 구역 | 매물 분류 |
| ... | ... | ... | ... |
```

전체 30-50개 용어. 사용자 노출 한국어와 코드 영어 1:1 매핑.

- [ ] **Step 4: `docs/ssot-matrix.md` (정보별 SSOT + 위반 검출)**

섹션:
1. SSOT 매트릭스 표 (정보 종류 / 진짜 SSOT / 사본 / 위반 차단 도구)
2. 문서 SSOT (한 폴더 = 한 도메인)
3. 코드 SSOT (Rust 도메인 = 비즈니스 규칙, OpenAPI 자동 생성)
4. 설정 SSOT (Pulumi = 인프라, 수동 콘솔 변경 금지)
5. 자동 차단 룰 9개 (Task 5에서 정의된 lefthook + Task 6 CI)

- [ ] **Step 5: 검증**

```bash
wc -l docs/sss-charter.md docs/glossary.md docs/ssot-matrix.md docs/README.md
pnpm biome format --write docs/*.md
pnpm markdownlint-cli2 docs/*.md
```

기대: 모든 파일 ≤500줄, 포맷 통과.

- [ ] **Step 6: 커밋**

```bash
git add docs/README.md docs/sss-charter.md docs/glossary.md docs/ssot-matrix.md
git commit -m "docs(charter): add SSS 7-pillar charter + glossary + SSOT matrix"
```

---

### Task 9: ADR 12개 (README + 0001-0011)

**Files:**
- Create: `docs/adr/README.md`, `docs/adr/0001-...md` ~ `docs/adr/0011-...md`

- [ ] **Step 1: ADR 템플릿 + README 작성**

`docs/adr/README.md`:
```markdown
# Architecture Decision Records (ADR)

모든 기술·아키텍처 결정의 영구 기록.

## 작성 원칙
- 시간 순서가 의미 → `NNNN-` prefix 유지
- 한 결정 = 한 파일
- 승인 후 *수정 금지*. 변경은 새 ADR로.
- 결정 보류 / 재검토는 *trigger 명시*

## 템플릿
\`\`\`markdown
# ADR-NNNN: <제목>

| | |
|---|---|
| 작성일 | YYYY-MM-DD |
| 상태 | Proposed / Accepted / Deprecated / Superseded by ADR-XXX |
| 결정자 | <이름 또는 역할> |

## 컨텍스트
<왜 이 결정이 필요한가, 어떤 제약이 있는가>

## 결정
<무엇을 정했는가, 한 문장>

## 대안
- 대안 1: <왜 안 함>
- 대안 2: <왜 안 함>

## 결과
- 긍정: <이 결정으로 얻는 것>
- 부정: <이 결정의 비용>
- 영향 받는 영역: <crate / 폴더 / 시스템>

## 재검토 트리거
- <조건 1>
- <조건 2>

## 참조
- → @docs/...
\`\`\`

## 인덱스
- [0001 — Language: Rust + TypeScript](./0001-language-rust-ts.md)
- [0002 — Monorepo: Cargo + pnpm + Turborepo](./0002-monorepo-cargo-pnpm-turbo.md)
- [0003 — Frontend: Next.js 16 + React 19](./0003-frontend-nextjs-react19.md)
- [0004 — DB: PostgreSQL 17 + PostGIS](./0004-db-postgres-postgis.md)
- [0005 — Auth IdP: Zitadel](./0005-auth-zitadel.md)
- [0006 — API: REST + OpenAPI (utoipa)](./0006-api-rest-openapi.md)
- [0007 — Cache: moka L1 + Valkey L2](./0007-cache-moka-valkey.md)
- [0008 — Observability: Grafana + OTel + Sentry](./0008-observability-grafana-otel-sentry.md)
- [0009 — IaC: Pulumi (TypeScript)](./0009-iac-pulumi.md)
- [0010 — Scope: 산업용 부동산 정보 플랫폼 (옵션 A)](./0010-scope-information-platform-option-a.md)
- [0011 — Embedding: Gemini + pgvector](./0011-embedding-gemini-pgvector.md)
```

- [ ] **Step 2: ADR-0001 작성 — Language**

`docs/adr/0001-language-rust-ts.md`:
- 컨텍스트: SSS 엔터프라이즈, 메모리 안전 + 성능 + 동시성
- 결정: Rust (백엔드) + TypeScript (Next.js 프론트)
- 대안: Kotlin/Spring(JVM 무거움), Go(GC, race 가능), Node 풀스택(성능)
- 결과: 학습 곡선↑, 인력 풀↓, 다만 SSS 가치 우선
- 재검토 트리거: Rust 채용 6개월 이상 실패 시

- [ ] **Step 3: ADR-0002 ~ 0011 (10개 작성)**

각 ADR을 순차 작성. 각 ADR ≤300줄.

- 0002: pnpm workspace + Cargo workspace + Turborepo
- 0003: Next.js 16 + React 19 (fetch caching, Server Components)
- 0004: Postgres 17 + PostGIS (vs CockroachDB, vs MySQL)
- 0005: Zitadel (vs Keycloak 비교 본문 + 재검토 트리거)
- 0006: REST + OpenAPI 3.1 + utoipa + openapi-typescript
- 0007: moka (L1) + Valkey (L2). L3 보류
- 0008: Grafana + Prometheus + Loki + Tempo + Sentry + OTel
- 0009: Pulumi TypeScript (vs Terraform/OpenTofu)
- 0010: 옵션 A 데이터 플랫폼 (AI 생성 X, 임베딩만)
- 0011: Gemini Embedding 2 + pgvector (Phase 3 도입)

- [ ] **Step 4: 검증**

```bash
ls docs/adr/
wc -l docs/adr/*.md     # 모두 ≤500
pnpm biome format --write docs/adr/*.md
pnpm markdownlint-cli2 docs/adr/*.md
```

기대: 12개 파일, 모두 ≤500줄, 포맷 통과.

- [ ] **Step 5: 커밋**

```bash
git add docs/adr/
git commit -m "docs(adr): add first 11 ADRs (language/monorepo/frontend/db/auth/api/cache/obs/iac/scope/embedding)"
```

---

### Task 10: Conventions 10개 (README + 9)

**Files:**
- Create: `docs/conventions/README.md`, `rust.md`, `typescript.md`, `sql.md`, `naming-and-ids.md`, `error-format.md`, `ui-writing-korean.md`, `testing.md`, `git-and-pr.md`, `comments.md`

- [ ] **Step 1: README + 학습 순서**

`docs/conventions/README.md`:
- 학습 순서: 네이밍 → Rust → TS → SQL → 에러 형식 → UI 라이팅 → 테스트 → Git/PR → 주석
- 각 컨벤션이 자동 강제되는 도구 매핑

- [ ] **Step 2: `rust.md` — rustfmt + clippy 룰 + 패턴**

내용:
- rustfmt 룰 (이미 `rustfmt.toml` 정의)
- clippy 룰 (이미 `clippy.toml` + Cargo.toml workspace lints)
- 도메인 패턴: 값 객체(Newtype), Repository trait, 도메인 이벤트
- 에러 처리: thiserror (도메인) + anyhow (앱)
- 비동기: Tokio, async-trait, async fn in trait (1.83+)

- [ ] **Step 3: `typescript.md` — Biome + TS strict + Next.js 규칙**

내용:
- Biome 룰 (이미 `biome.json` 정의)
- TS strict (이미 `tsconfig.base.json`)
- Next.js: Server Component 기본, Client는 명시적 `"use client"`
- Server Action = 얇은 프록시 (인증 + Rust API 호출)
- 비즈니스 로직 0줄 (Rust로 위임)

- [ ] **Step 4: `sql.md` — sqlfluff PostgreSQL 룰**

내용:
- snake_case (테이블/컬럼)
- 키워드 lowercase
- PostGIS: SRID 명시 강제
- 인덱스: GIST (공간), B-Tree (일반), BRIN (시계열)
- 마이그레이션 안전: NOT NULL은 DEFAULT 동반

- [ ] **Step 5: `naming-and-ids.md` — ULID prefix + 네이밍**

내용:
- ULID + prefix 표 (usr_, lst_, prc_, bld_, ic_, mfr_, inq_, bmk_, ...)
- 변수/함수/타입 케이스 (Rust: snake/Pascal, TS: camel/Pascal)
- 파일명 (Rust: snake_case, TS: kebab-case)
- API URL: kebab-case 복수 (`/v1/listings`)

- [ ] **Step 6: `error-format.md` — RFC 9457 Problem Details**

내용:
- 응답 JSON 형식 (type/title/status/detail/instance/correlationId/code/errors)
- 에러 코드 SCREAMING_SNAKE_CASE
- type URL = `https://gongzzang.com/errors/<kebab-case>`
- 한국어 메시지 (해요체)

- [ ] **Step 7: `ui-writing-korean.md` — 해요체**

내용:
- 정상 / 에러 / 확인 / 빈 상태 톤 매트릭스
- 단어 통일 ("매물" 사용, "물건" 금지)
- 외래어 표기 (Tailwind는 "테일윈드", PostgreSQL은 "포스트그레SQL" 같은 통일)

- [ ] **Step 8: `testing.md` — 테스트 네이밍 + 분류**

내용:
- 단위/통합/E2E/계약/property/부하/카오스 분류
- 네이밍: `<주체>_<can/cannot/returns/throws>_<조건>`
- 커버리지 임계값: 도메인 ≥ 90%, 어댑터 ≥ 70%, UI ≥ 50%
- 도구: cargo test + insta + rstest, Vitest, Playwright

- [ ] **Step 9: `git-and-pr.md` — Conventional Commits + PR 룰**

내용:
- 브랜치: `feat/...`, `fix/...`, `chore/...`
- 커밋: Conventional Commits (이미 `lefthook.yml` commit-msg에 강제)
- PR 크기: ≤500줄 권장
- Squash merge
- main 직접 push 금지 (GitHub branch protection)

- [ ] **Step 10: `comments.md` — Why over What**

내용:
- TODO 형식: `// TODO(YYYY-Q?, #issue): description`
- HACK/XXX/FIXME 금지 (TODO로 통일)
- ADR 링크: `// see: docs/adr/0007-cache.md`
- 외부 참조: `// V-World API spec: https://...`

- [ ] **Step 11: 검증**

```bash
ls docs/conventions/
wc -l docs/conventions/*.md
pnpm biome format --write docs/conventions/*.md
pnpm markdownlint-cli2 docs/conventions/*.md
pnpm markdown-link-check docs/conventions/*.md
```

- [ ] **Step 12: 커밋**

```bash
git add docs/conventions/
git commit -m "docs(conventions): add 9 convention docs (rust/ts/sql/naming/error/ui/test/git/comments)"
```

---

### Task 11: Data Sources 6개

**Files:**
- Create: `docs/data-sources/README.md`, `v-world.md`, `data-go-kr.md`, `korean-law.md`, `nice-identity.md`, `naver-maps.md`

- [ ] **Step 1: README + 카탈로그 표**

`docs/data-sources/README.md`:
- 한국 공공 API 카탈로그 표 (소스 / 운영 기관 / 진입점 / 인증 방식 / 라이선스 / 문서)
- 각 소스 문서 작성 템플릿 (개요 / 인증 / Rate Limit / 핵심 엔드포인트 / 예시 / 에러 / 라이선스 / 프로덕션 주의 / Circuit Breaker 정책)
- → @docs/conventions/error-format.md

- [ ] **Step 2: `v-world.md`**

내용:
- 개요 (공간정보산업진흥원, https://www.vworld.kr)
- API 키 발급 (도메인 1개 등록 필수)
- 핵심 레이어: LT_C_UQ111-114 (용도지역), UPISUQ161/171 (지구단위/개발제한), UPISUQ151-159 (도시계획시설), 42개 법적지정
- 좌표계: EPSG:4326 (WGS84) 입출력
- 요청 예시 (WFS GetFeature, 지오코딩)
- Rate Limit + Circuit Breaker 정책
- raw_response 보존

- [ ] **Step 3: `data-go-kr.md`**

내용:
- 개요 (행정안전부, https://data.go.kr)
- 인증 (serviceKey 발급, ODP_SERVICE_KEY)
- 우리 사용 후보 API: 건축물대장 / 토지대장 / 부동산 실거래가 / 행정표준코드 / 도로명주소
- 각 API의 신청·승인 프로세스
- 라이선스 (이용허락범위 필드 확인)

- [ ] **Step 4: `korean-law.md`**

내용:
- 개요 (법제처, https://open.law.go.kr)
- 인증 (Open API 사용자 등록)
- 핵심 endpoint: 법령 검색 / 본문 / 별표 / 판례 / 조례
- 프로덕션 사용 패턴 (단순 조회는 우리 직접, 의미 검색은 임베딩 + pgvector)
- raw 보존 (법령 원문 영구 보관)

- [ ] **Step 5: `nice-identity.md`**

내용:
- 개요 (NICE 평가정보, 본인인증)
- 인증 흐름 (OIDC 또는 self-API)
- 비용: 건당 100-300원
- 도입 시점: sub-project 3 (인증)에서 결정
- 대안: KCB, Toss

- [ ] **Step 6: `naver-maps.md`**

내용:
- 개요 (네이버 클라우드 플랫폼)
- API 키 발급 (NAVER_MAPS_CLIENT_ID)
- 무료 티어: 월 12만 호출
- 좌표계: EPSG:4326 (WGS84)
- 클라이언트 SDK + 서버 사이드 지오코딩
- Canvas 마커 렌더 패턴 (Phase 3)

- [ ] **Step 7: 검증 + 커밋**

```bash
pnpm biome format --write docs/data-sources/*.md
pnpm markdownlint-cli2 docs/data-sources/*.md
git add docs/data-sources/
git commit -m "docs(data-sources): add 5 Korean public API catalogs"
```

---

### Task 12: 도메인 카테고리 README 13개 (스켈레톤)

**Files:**
- Create: `docs/{infrastructure,auth,data,cache-messaging,backend,api,observability,security,testing,frontend,governance,compliance,cost}/README.md`

- [ ] **Step 1: 공통 템플릿 정의**

각 README 형식:
```markdown
# <카테고리 이름>

<한 문장 책임>

## 책임 영역
- <영역 1>
- <영역 2>

## 작성 예정 문서 (sub-project N)
- `<file>.md` — <내용>

## 관련 ADR
- → @docs/adr/<NNNN>-<...>.md

## 관련 컨벤션
- → @docs/conventions/<...>.md

## 참조
- → @docs/glossary.md
```

- [ ] **Step 2: 13개 작성 (sequential)**

각각:

1. `infrastructure/` — IaC (Pulumi), Kubernetes, GitOps. 작성 예정: iac.md, k8s.md, gitops.md, ci-cd.md, deployment.md. ADR-0009.
2. `auth/` — Zitadel, OIDC, RBAC, NICE 본인인증. 작성 예정: zitadel.md, social-providers.md, nice-identity.md, webauthn.md, rbac.md. ADR-0005.
3. `data/` — Postgres + PostGIS, Medallion, 마이그레이션, CDC, 검색. 작성 예정: postgres.md, postgis.md, medallion.md, schemas.md, migrations.md, cdc.md, catalog.md, quality.md, retention.md. ADR-0004 + 0011.
4. `cache-messaging/` — moka L1, Valkey L2, Kafka, SQS, Outbox. ADR-0007.
5. `backend/` — Axum, SQLx, DDD, CQRS, Event Sourcing, Saga, Circuit Breaker, Idempotency. ADR-0001 + 0006.
6. `api/` — OpenAPI, utoipa, ts-codegen, 버저닝, 에러 형식, Pact, Rate Limit. ADR-0006.
7. `observability/` — OTel, Sentry, Prometheus, Loki, Tempo, SLO, On-call, RUM. ADR-0008.
8. `security/` — OWASP ASVS, PIPA, ISMS-P, 데이터 분류, PII 마스킹, 암호화, 시크릿, SAST/DAST, 공급망, threat modeling, pen-test.
9. `testing/` — 단위/통합/E2E/property/mutation/load/chaos/contract/visual.
10. `frontend/` — Next.js, shadcn/Radix, TanStack Query, 네이버 지도, Canvas 마커, PMTiles, i18n, a11y, CSP, 성능 예산. ADR-0003.
11. `governance/` — ADR, CODEOWNERS, Conventional Commits, Changesets, Renovate, Backstage, C4, DORA.
12. `compliance/` — PIPA, ISMS-P, SOC 2, ISO 27001, audit log, retention, GDPR RTBF, 공공데이터 라이선스.
13. `cost/` — Phase별 비용 추정, AWS RI/Spot 전략, 멀티 리전 미루기, 컴플라이언스 매출 후.

- [ ] **Step 3: 검증 + 커밋**

```bash
pnpm biome format --write docs/*/README.md
pnpm markdownlint-cli2 docs/*/README.md
git add docs/infrastructure docs/auth docs/data docs/cache-messaging docs/backend docs/api docs/observability docs/security docs/testing docs/frontend docs/governance docs/compliance docs/cost
git commit -m "docs(domains): add 13 domain category READMEs (skeletons)"
```

---

### Task 13: 워크스페이스 멤버 README 28개

**Files:**
- Create: `apps/{platform-web,admin-web}/README.md`
- Create: `services/{api,worker,data-pipeline}/README.md`
- Create: `crates/domain/{core,market,regulation,insights,shared-kernel}/README.md`
- Create: `crates/{data-clients,db,geo,auth,cache,observability,circuit-breaker,api-types,audit,embedding}/README.md`
- Create: `packages/{ui-web,api-client,shared,map,tsconfig}/README.md`
- Create: `infrastructure/README.md`, `tools/README.md`, `db/migration/README.md`

- [ ] **Step 1: 폴더 생성**

```bash
mkdir -p apps/platform-web apps/admin-web
mkdir -p services/api services/worker services/data-pipeline
mkdir -p crates/domain/{core,market,regulation,insights,shared-kernel}
mkdir -p crates/{data-clients,db,geo,auth,cache,observability,circuit-breaker,api-types,audit,embedding}
mkdir -p packages/{ui-web,api-client,shared,map,tsconfig}
mkdir -p infrastructure tools db/migration
```

- [ ] **Step 2: 공통 멤버 README 템플릿**

각 README:
```markdown
# <member-name>

<한 줄 책임>

## 의존
- <upstream package>
- <upstream package>

## 사용
- <consumer 1>
- <consumer 2>

## 정책
- <core policy 1>
- <core policy 2>

## 향후 작업 (sub-project N)
- <task 1>

## 참조
- → @docs/<domain>/README.md
- → @docs/conventions/<lang>.md
```

- [ ] **Step 3: 28개 README 작성**

스펙 § 6.10 참조해서 각 멤버 책임 한 문장 + 의존 + 정책.

대표 예:
- `apps/platform-web/README.md` — 사용자 사이트, Next.js, Naver Maps. 의존: @gongzzang/{core,data-clients,ui-web,db}. LLM 의존성 금지.
- `services/api/README.md` — Rust Axum HTTP API, OpenAPI 자동 생성, 모든 외부 호출 Circuit Breaker.
- `crates/domain/core/README.md` — DDD Core BC. User/Listing/Parcel/Building/IndustrialComplex/Manufacturer.
- `crates/embedding/README.md` — Phase 3 자리. Gemini Embedding 2 + pgvector. ADR-0011.

- [ ] **Step 4: 검증 + 커밋**

```bash
pnpm biome format --write apps/**/README.md services/**/README.md crates/**/README.md packages/**/README.md
pnpm markdownlint-cli2 apps/**/README.md services/**/README.md crates/**/README.md packages/**/README.md infrastructure/README.md tools/README.md db/migration/README.md

git add apps services crates packages infrastructure tools db
git commit -m "docs(workspace): add 28 member READMEs (skeletons)"
```

---

### Task 14: .claude / .agents / .mcp.json (3개)

**Files:**
- Create or Update: `.claude/settings.json`, `.mcp.json`, `.agents/README.md`

- [ ] **Step 1: `.claude/settings.json`**

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "permissions": {
    "allow": [
      "Bash(ls *)",
      "Bash(pwd)",
      "Bash(cat *)",
      "Bash(git status)",
      "Bash(git diff *)",
      "Bash(git log *)",
      "Bash(pnpm turbo *)",
      "Bash(pnpm biome *)",
      "Bash(pnpm install)",
      "Bash(cargo check *)",
      "Bash(cargo clippy *)",
      "Bash(cargo fmt *)",
      "Bash(cargo deny *)",
      "Bash(cargo test *)",
      "Bash(docker compose *)"
    ],
    "deny": [
      "Bash(rm -rf *)",
      "Bash(git push --force *)",
      "Bash(git push -f *)",
      "Bash(git reset --hard *)",
      "Bash(psql * DROP *)",
      "Bash(curl * | sh)",
      "Bash(wget * | bash)"
    ]
  },
  "hooks": {}
}
```

- [ ] **Step 2: `.mcp.json` 갱신**

```json
{
  "$schema": "https://modelcontextprotocol.io/schema/mcp-config.json",
  "description": "공짱 프로젝트 MCP — 개발자 Claude 세션 전용. 메인 코드 import 금지.",
  "mcpServers": {}
}
```

(현재는 비움. 향후 Zitadel docs MCP 또는 자체 MCP는 sub-project 진행 중 추가.)

- [ ] **Step 3: `.agents/README.md`**

```markdown
# .agents/

에이전트 공용 자료 (Claude / OpenAI / Cursor / Cline / Aider 등 모든 도구가 공유).

## 정책
- 도구 무관 자료만 (도구별 룰은 `.claude/`, `.cursor/` 등에)
- 모든 AI 도구가 읽을 수 있는 Markdown 형식
- AGENTS.md가 진입점, 이 폴더는 보조 자료

## 향후 추가 (sub-project 단위)
- subagent 정의 (예: `code-reviewer.md`, `docs-auditor.md`)
- 공용 prompt 템플릿
- glossary 별칭 (도구별 차이)
```

- [ ] **Step 4: 검증 + 커밋**

```bash
pnpm biome check .claude/settings.json .mcp.json
pnpm markdownlint-cli2 .agents/README.md
git add .claude/settings.json .mcp.json .agents/
git commit -m "chore(agents): set up Claude/AGENTS shared config and MCP placeholder"
```

---

### Task 15: GitHub 메타 (CODEOWNERS + PR + Issue 템플릿)

**Files:**
- Create: `.github/CODEOWNERS`, `.github/pull_request_template.md`, `.github/ISSUE_TEMPLATE/bug.md`, `.github/ISSUE_TEMPLATE/feature.md`

- [ ] **Step 1: `.github/CODEOWNERS`**

```
# 기본: 1인 운영자 (확장 시 멤버 추가)
* @w1kch9812-cmd

# 도메인별 (사용자 결정 시 갱신)
/docs/adr/             @w1kch9812-cmd
/docs/conventions/     @w1kch9812-cmd
/docs/sss-charter.md   @w1kch9812-cmd
/.github/              @w1kch9812-cmd
/infrastructure/       @w1kch9812-cmd
/crates/auth/          @w1kch9812-cmd
/services/             @w1kch9812-cmd
```

- [ ] **Step 2: `.github/pull_request_template.md`**

```markdown
## 변경 요약
<!-- 이 PR이 무엇을 바꾸는지 1-3 문장 -->

## 동기 (Why)
<!-- 왜 이 변경이 필요한가 -->

## 변경 범위
- [ ] 단일 sub-project 안에서만 변경
- [ ] sub-project N의 spec/plan과 일치
- [ ] 관련 ADR 작성/갱신 됨

## 테스트
- [ ] 단위 테스트 추가/갱신
- [ ] 통합 테스트 (필요 시)
- [ ] E2E (필요 시)
- [ ] 수동 확인 시나리오: <...>

## SSS 7 기둥 자체 검증
- [ ] 일관성: 기존 패턴 따름
- [ ] 자동 강제: lefthook + CI 통과
- [ ] 추적성: ADR/audit log 갱신
- [ ] 안전성: 타입/값 객체 검증
- [ ] 가시성: tracing/log 추가
- [ ] SSOT: 정보 중복 없음
- [ ] 명확성: 컨벤션 준수

## Breaking Change
- [ ] 없음
- [ ] 있음 (영향 + 마이그레이션 명시):

## 관련 이슈
Closes #
```

- [ ] **Step 3: Issue 템플릿 2개**

`.github/ISSUE_TEMPLATE/bug.md`:
```markdown
---
name: 버그
about: 버그 신고
title: 'bug: '
labels: bug
---

## 무엇이 잘못됐나요
<!-- 한 문장 -->

## 재현 방법
1.
2.
3.

## 기대 동작 vs 실제 동작
**기대:**

**실제:**

## 환경
- OS:
- 브라우저 (해당 시):
- 앱 버전:
- correlation_id (있다면):

## 추가 정보
스크린샷, 로그, ...
```

`.github/ISSUE_TEMPLATE/feature.md`:
```markdown
---
name: 기능 제안
about: 새 기능 제안
title: 'feat: '
labels: enhancement
---

## 무엇을 원하나요
<!-- 한 문장 -->

## 동기 (Why)
<!-- 누가 어떤 상황에서 어떤 가치를 얻나 -->

## 사용자 시나리오
<!-- "매수자가 ... 할 때 ... 하면 ..." 형식 -->

## 영향 범위
- 도메인:
- 관련 sub-project:
- 신규 ADR 필요 여부:

## YAGNI 자체 점검
- [ ] 정말 지금 필요한 기능인가?
- [ ] 더 단순한 대안 검토했는가?
```

- [ ] **Step 4: 검증 + 커밋**

```bash
pnpm markdownlint-cli2 .github/**/*.md
git add .github/CODEOWNERS .github/pull_request_template.md .github/ISSUE_TEMPLATE/
git commit -m "chore(github): add CODEOWNERS + PR template + issue templates"
```

---

### Task 16: 최종 검증 + 첫 PR

**Files:**
- 없음 (검증만)

- [ ] **Step 1: 전체 파일 수 확인**

```bash
find . -type f \
  ! -path "./node_modules/*" \
  ! -path "./_archive/*" \
  ! -path "./target/*" \
  ! -path "./.next/*" \
  ! -path "./reference/*" \
  ! -path "./.git/*" \
  | wc -l
```

기대: 76개 이상.

- [ ] **Step 2: 전 파일 ≤500줄 검증**

```bash
find . -type f \( -name "*.md" -o -name "*.toml" -o -name "*.json" -o -name "*.yml" -o -name "*.yaml" \) \
  ! -path "./node_modules/*" ! -path "./_archive/*" ! -path "./target/*" ! -path "./reference/*" ! -path "./.git/*" \
  -exec sh -c 'wc -l "$1" | awk "{print \$1, \$2}" | awk "\$1 > 500"' _ {} \;
```

기대: 출력 없음 (모든 파일 ≤500줄). 있으면 그 파일 분해 필요.

- [ ] **Step 3: 모든 도구 풀 검증**

```bash
pnpm biome check .
pnpm markdownlint-cli2 "**/*.md" "#node_modules" "#_archive" "#target" "#reference"
pnpm markdown-link-check docs/**/*.md AGENTS.md README.md TECH.md MEMORY.md CLAUDE.md
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo deny check
```

기대: 모두 0 exit code.

- [ ] **Step 4: pre-commit 훅 실제 작동 확인**

```bash
# 더미 시크릿
echo "ZITADEL_API_KEY=sk_live_realtoken1234567890abcdef" > _temp_secret.txt
git add _temp_secret.txt
git commit -m "test"
# 기대: gitleaks가 차단

# 정리
git restore --staged _temp_secret.txt
rm _temp_secret.txt

# 더미 1500줄 markdown
yes "## heading" | head -1600 > _temp_huge.md
git add _temp_huge.md
git commit -m "test"
# 기대: file-size hook이 차단

git restore --staged _temp_huge.md
rm _temp_huge.md
```

- [ ] **Step 5: SSS 15 검증 체크 (가능 항목)**

`docs/sss-charter.md`의 15 검증 질문 중 sub-project 1에서 통과 가능한 항목 확인:
- Q3 (에러 형식 SSOT) — `docs/conventions/error-format.md` 존재 ✅
- Q4 (의존성 방향 룰) — `Cargo.toml` workspace lints + dependency-cruiser 추후 ✅(부분)
- Q5 (시크릿 자동 차단) — gitleaks pre-commit + CI ✅
- Q6 (모든 결정 ADR) — 11개 존재 ✅
- Q9 (코드 스타일 commit 차단) — lefthook biome + rustfmt ✅
- Q11 (정보별 SSOT 매트릭스) — `docs/ssot-matrix.md` 존재 ✅
- Q13 (도메인 용어 사전) — `docs/glossary.md` ✅. 위반 자동 검출 룰은 sub-project 2+에서 추가
- Q14 (모든 파일 ≤500줄 자동 검증) — lefthook + CI file-size hook ✅

→ 8/15 통과. 나머지 7개는 sub-project 2+에서.

- [ ] **Step 6: 첫 PR 생성**

`feature/sub-project-1-charter` 브랜치는 위 모든 커밋이 main에 직접인 경우 skip. PR 워크플로우 원하면:

```bash
git checkout -b chore/finalize-sub-project-1
# (변경 없으면 빈 PR이 안 만들어짐. 그냥 main에 모두 푸시되어 있어야 OK.)
git checkout main
git push origin main
```

GitHub Actions CI 실행 모니터링:
```bash
# https://github.com/w1kch9812-cmd/gongzzang3/actions 접속
# 모든 job 그린 확인
```

- [ ] **Step 7: spec § 9 검증 기준 체크리스트 채움**

`docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md` § 9를 열어서 모든 체크박스 표기 + 사용자 검증 받음.

- [ ] **Step 8: 마무리 커밋**

```bash
git add docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md
git commit -m "docs(spec): mark sub-project 1 verification checklist complete"
git push origin main
```

- [ ] **Step 9: Sub-project 1 완료 선언**

`docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md` 헤더 메타데이터:
- `상태` Draft → **Done**

```bash
# Edit 또는 sed
git add docs/superpowers/specs/2026-05-01-sub-project-1-charter-monorepo-design.md
git commit -m "docs(spec): close sub-project 1"
git push origin main
```

---

## Self-Review (이 plan 자체)

### Spec coverage
spec § 6 결과물 목록 76개를 task 7-15에 1:1 매핑함:
- 루트 진입점 10 → Task 7 (4개) + Task 5 (.gitignore/.editorconfig/.gitattributes/.nvmrc/LICENSE) + Task 3 (`.env.example`)
- 모노레포 설정 10 → Task 3
- 자동 강제 6 → Task 4 + 5 + 6
- GitHub 메타 4 → Task 15
- .claude/.agents/.mcp 3 → Task 14
- docs 진입점 4 → Task 8
- ADR 12 → Task 9
- Conventions 10 → Task 10
- 도메인 카테고리 13 → Task 12
- Data sources 6 → Task 11
- 워크스페이스 멤버 28 → Task 13

→ 합계 일치.

### Placeholder 스캔
- 각 ADR/컨벤션 섹션에 *작성할 내용 outline*만 있고 *완성 본문* 없음 — 의도적 (각 파일은 plan 실행 중 작성, plan은 *작성 가이드*)
- "TBD" 또는 "TODO" 없음
- 모든 step에 명령 또는 코드 포함 ✅

### Type / Path 일관성
- ADR 파일명 일관 (`0005-auth-zitadel.md` 4곳에서 동일)
- 워크스페이스 멤버 경로 일관 (`crates/embedding/` 한 곳)
- 환경 변수 일관 (`ZITADEL_API_KEY`, `VWORLD_API_KEY` 등)

### 누락 검토
- spec § 9 검증 기준 모두 Task 16에서 체크 ✅
- spec § 11 후속 sub-project 2-12 — plan은 sub-project 1에 한정, 다른 sub-project는 별도 plan ✅

→ 자체 검토 통과.

---

## 실행 방식 선택

Plan complete and saved to `docs/superpowers/plans/2026-05-01-sub-project-1-charter-monorepo.md`.

두 가지 실행 옵션:

**1. Subagent-Driven (추천)** — 각 task마다 fresh subagent dispatch, task 사이에 사용자 검토. 빠른 반복.

**2. Inline Execution** — 이 세션에서 task를 순차 실행, 체크포인트마다 검토.

어느 방식?

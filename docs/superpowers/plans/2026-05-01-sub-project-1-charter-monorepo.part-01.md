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

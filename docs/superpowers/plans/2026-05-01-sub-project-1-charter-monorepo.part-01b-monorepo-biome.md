# Sub-project 1 - Part 01B: Monorepo Root And Biome Setup

Parent index: [Sub-project 1 Part 01](./2026-05-01-sub-project-1-charter-monorepo.part-01.md).

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

# Sub-project 1 - Part 01C: Enforcement Tooling Setup

Parent index: [Sub-project 1 Part 01](./2026-05-01-sub-project-1-charter-monorepo.part-01.md).

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

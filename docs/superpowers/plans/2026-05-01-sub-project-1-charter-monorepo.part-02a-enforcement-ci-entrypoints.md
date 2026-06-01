# Sub-project 1 - Part 02A: Enforcement Completion, CI, And Entry-Point Docs

Parent index: [Sub-project 1 Part 02](./2026-05-01-sub-project-1-charter-monorepo.part-02.md).
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

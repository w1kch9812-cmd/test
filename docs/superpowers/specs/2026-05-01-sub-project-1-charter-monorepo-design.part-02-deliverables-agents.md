## 6. 결과물 목록 (60-80개 파일)

### 6.1 루트 진입점 (7개)

- `AGENTS.md` — 오픈 표준 라우터, 7 기둥 요약, 작업별 → 도메인 매핑
- `CLAUDE.md` — `Read @AGENTS.md` 1줄
- `README.md` — 사람용 프로젝트 소개 (≤200줄)
- `TECH.md` — 기술 스택 한눈 + SSOT 맵
- `MEMORY.md` — 자동 메모리 인덱스 (기존)
- `.mcp.json` — MCP 서버 설정
- `.gitignore`, `.editorconfig`, `.gitattributes`, `.nvmrc`

### 6.2 SSS 헌법 + SSOT (3개)

- `docs/sss-charter.md` — 7 기둥 + 15 검증 (≤500줄)
- `docs/glossary.md` — 한·영 용어 사전
- `docs/ssot-matrix.md` — 정보별 SSOT + 위반 자동 차단 룰

### 6.3 ADR (12개: README + 11개)

- `docs/adr/README.md` — ADR 인덱스 + 작성 가이드
- `docs/adr/0001-language-rust-ts.md`
- `docs/adr/0002-monorepo-cargo-pnpm-turbo.md`
- `docs/adr/0003-frontend-nextjs-react19.md`
- `docs/adr/0004-db-postgres-postgis.md`
- `docs/adr/0005-auth-zitadel.md` — Zitadel 채택. Keycloak 비교 + 재검토 트리거 본문 포함
- `docs/adr/0006-api-rest-openapi.md`
- `docs/adr/0007-cache-moka-valkey.md`
- `docs/adr/0008-observability-grafana-otel-sentry.md`
- `docs/adr/0009-iac-pulumi.md`
- `docs/adr/0010-scope-information-platform-option-a.md`
- `docs/adr/0011-embedding-gemini-pgvector.md`

각 ADR 형식: 컨텍스트 / 결정 / 대안 / 결과 (200-400줄).

### 6.4 컨벤션 (10개: README + 9개)

- `docs/conventions/README.md`
- `docs/conventions/rust.md` — rustfmt + clippy pedantic + clippy.toml 룰
- `docs/conventions/typescript.md` — Biome v2.4 (단독 도구, 보안 룰 부족분은 Semgrep + Snyk + gitleaks로 별도 보강)
- `docs/conventions/sql.md` — sqlfluff PostgreSQL 방언
- `docs/conventions/naming-and-ids.md` — ULID prefix 표 + 네이밍 규칙
- `docs/conventions/error-format.md` — RFC 9457 Problem Details
- `docs/conventions/ui-writing-korean.md` — 해요체, 한국어 가이드
- `docs/conventions/testing.md` — 테스트 네이밍 + 분류
- `docs/conventions/git-and-pr.md` — Conventional Commits + PR 룰
- `docs/conventions/comments.md` — Why over What, TODO 형식

### 6.5 도메인 카테고리 README 스켈레톤 (13개)

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

각 README는 *목차 + 다른 sub-project에서 채울 .md 목록 + 책임 한 문장*.

### 6.6 외부 데이터 소스 (6개)

- `docs/data-sources/README.md`
- `docs/data-sources/v-world.md`
- `docs/data-sources/data-go-kr.md`
- `docs/data-sources/korean-law.md`
- `docs/data-sources/nice-identity.md`
- `docs/data-sources/naver-maps.md`

### 6.7 모노레포 설정 (10개)

- `Cargo.toml` (workspace + 공유 deps + 공유 lints)
- `rust-toolchain.toml` (Rust 1.83+)
- `clippy.toml`, `rustfmt.toml`, `deny.toml`
- `package.json` (root, pnpm 9+)
- `pnpm-workspace.yaml`
- `turbo.json`
- `tsconfig.base.json`
- `biome.json` (Biome v2.4 단독 — format + lint + import sort 통합)
- `.env.example`

### 6.8 자동 강제 인프라 (5개)

- `lefthook.yml` — pre-commit + pre-push 훅
- `.gitleaks.toml` — 시크릿 스캔
- `markdownlint.json`
- `renovate.json`
- `.github/workflows/ci.yml` — lint/format/typecheck/test/secret-scan/sca/sbom

### 6.9 GitHub 메타 (4개)

- `.github/CODEOWNERS`
- `.github/pull_request_template.md`
- `.github/ISSUE_TEMPLATE/bug.md`
- `.github/ISSUE_TEMPLATE/feature.md`

### 6.10 워크스페이스 멤버 README 스켈레톤 (~25개)

- `apps/{platform-web,admin-web}/README.md`
- `services/{api,worker,data-pipeline}/README.md`
- `crates/domain/{core,market,regulation,insights,shared-kernel}/README.md`
- `crates/{data-clients,db,geo,auth,cache,observability,circuit-breaker,api-types,audit,embedding}/README.md`
- `packages/{ui-web,api-client,shared,map,tsconfig}/README.md`
- `infrastructure/README.md`, `tools/README.md`, `reference/README.md`
- `tests/{e2e,load,chaos,contract}/README.md`
- `db/migration/README.md`

### 6.11 .claude / .agents (3-5개)

- `.claude/settings.json`
- `.claude/hooks/` (스켈레톤)
- `.agents/` (subagent 정의 자리)

### 6.12 루트 docs README (1개)

- `docs/README.md` — 카테고리 목차 + 학습 순서

**총 약 70-80개 파일.** 모두 ≤500줄, 단일 책임.

---

## 7. AGENTS.md 핵심 구조

이 파일이 SSOT 라우터. 모든 AI 도구가 이걸 먼저 읽음.

```markdown
# AGENTS.md

## 0. 7 기둥 SSS 헌법 (요약)
- 일관성, 자동 강제, 추적성, 안전성, 가시성, SSOT, 명확성
- 상세: → @docs/sss-charter.md

## 1. 절대 규칙
- 모든 파일 ≤500줄
- @docs/glossary.md 의 용어만 사용
- LLM 생성 텍스트를 사용자에게 직접 노출 금지 (옵션 A)
- 임시방편 금지 (`TEMP`, `HACK`, `XXX` 코드 차단)

## 2. 작업별 진입점
| 작업 | 우선 참조 |
|------|----------|
| 새 기능 | @docs/backend/ + @docs/conventions/ |
| 새 외부 API | @docs/data-sources/ + @docs/backend/circuit-breaker.md |
| DB 스키마 | @docs/data/schemas.md + @docs/data/migrations.md |
| 인증 작업 | @docs/auth/ + @docs/conventions/error-format.md |
| UI 컴포넌트 | @docs/frontend/ + @docs/conventions/ui-writing-korean.md |
| 인프라 변경 | @docs/infrastructure/iac.md (Pulumi 수동 콘솔 변경 금지) |
| 새 결정 | @docs/adr/README.md (ADR 작성 후 코드) |

## 3. 자동 강제 (참고)
- pre-commit: lefthook → format + lint + secret scan
- CI: lint + typecheck + test + SBOM + 이미지 서명
- 통과 못하면 머지 불가

## 4. 한국어 규칙
- 사용자 노출 텍스트: 해요체
- 에러: 원인 + 대응 안내
- 법령 인용: 정식 명칭 + 조·항·호 (단, AI 생성 금지)

## 5. 데이터 접근 규칙
- 메인 시스템: 공식 API 직접 호출
- AI 어시스턴트(향후): MCP 사용 가능
- 두 경로 격리

## 6. 1500줄 안티패턴 경보
- 500줄 초과 시 폴더로 분해
- 1500줄 = 자동 차단 (CI)
```

≤300줄 유지.

---

## 8. 자동 강제 흐름 (5단계)

| 단계 | 도구 | 차단 대상 |
|------|------|---------|
| 1. 에디터 | rust-analyzer, Biome 확장 | 실시간 lint/format |
| 2. pre-commit | lefthook + gitleaks | format, 빠른 lint, 시크릿 |
| 3. pre-push | lefthook | typecheck, 단위 테스트, 의존성 방향 |
| 4. CI (PR) | GitHub Actions | 풀스택 검증 (lint/type/test/SAST/SCA/OpenAPI diff/파일 크기) |
| 5. CI (merge) | GitHub Actions | SBOM 생성 + Cosign 서명 |

`lefthook.yml`, `.github/workflows/ci.yml`이 이 흐름의 SSOT.

---

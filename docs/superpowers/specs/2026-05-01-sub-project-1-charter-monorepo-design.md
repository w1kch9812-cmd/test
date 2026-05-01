# Sub-project 1 — 프로젝트 헌법 + 모노레포 셋업

| | |
|---|---|
| **작성일** | 2026-05-01 |
| **상태** | Draft (사용자 검토 대기) |
| **타입** | Foundation (모든 후속 sub-project의 기반) |
| **소요 추정** | 1-2주 |
| **포함 코드** | 0줄 (서비스 구현은 sub-project 2+) |
| **결과물 수** | 60-80개 파일 |

---

## 1. 목적 (Why)

이 프로젝트(공짱 — 산업용 부동산 정보 플랫폼)가 **하이엔드 엔터프라이즈 SSS급**으로 성립하려면, 코드 한 줄을 쓰기 전에 다음이 필요합니다:

1. **SSS의 정의** — 측정 가능한 7 기둥 헌법
2. **모든 후속 결정의 측정 자** — 컨벤션 + SSOT 매트릭스
3. **자동 강제 인프라** — 사람이 깜빡할 수 있는 모든 규칙을 시스템이 차단
4. **트리 구조 SSOT** — 정보의 단일 출처가 어디인지 명확
5. **결정 이력 (ADR)** — 지금까지 합의한 11개 결정 영구 보존
6. **모노레포 골격** — 후속 12개 sub-project가 들어올 자리

**Sub-project 1은 이 6개를 만든다**. 코드는 0줄. 문서·설정·CI 워크플로우만.

이게 정직하게 끝나야 sub-project 2 (DB 스키마 + Core 도메인) 부터의 모든 작업이 *측정 가능한 SSS* 기준 위에서 진행된다.

---

## 2. 범위 (Scope)

### 2.1 포함 (In Scope)

- 7 기둥 SSS 헌법 정의 (`docs/sss-charter.md`)
- 도메인 용어 사전 (`docs/glossary.md`)
- SSOT 매트릭스 (`docs/ssot-matrix.md`)
- 첫 11개 ADR
- 9개 컨벤션 문서 (`docs/conventions/`)
- 13개 도메인 카테고리 README 스켈레톤 (`docs/{infrastructure,auth,...}/README.md`)
- 5개 외부 데이터 소스 카탈로그 (`docs/data-sources/`)
- 모노레포 골격 (Cargo + pnpm workspace + Turborepo)
- 자동 강제 인프라 (lefthook + GitHub Actions CI)
- AGENTS.md 라우터 + CLAUDE.md 1줄 위임
- `.claude/` `.agents/` `.mcp.json` 셋업
- 각 워크스페이스 멤버 README 스켈레톤 (apps/*, services/*, crates/*, packages/*)

### 2.2 제외 (Out of Scope)

- 서비스 코드 (Rust/TS) — sub-project 2+
- DB 스키마 (마이그레이션 V1__init.sql 외) — sub-project 2
- 인증 구현 — sub-project 3
- 외부 API 통합 — sub-project 4
- API endpoint — sub-project 5
- UI 화면 — sub-project 6
- 인프라 프로비저닝 (Pulumi 코드는 sub-project 8) — 다만 폴더 자리는 잡음
- ISMS-P 인증 본격 추진 — Phase 3 후반

### 2.3 결정 보류 (Deferred Decisions)

이 sub-project에서는 결정하지 않고, 해당 sub-project 시작 시 다시 brainstorm:

- ECS Fargate vs EKS — sub-project 8 (인프라)
- 결제 게이트웨이 (Toss/I'mport/KG Inicis) — sub-project별 결제 도입 시
- NICE 본인인증 통합 시점 — sub-project 3
- 한국어 임베딩 전용 모델 미세조정 — Phase 4+

---

## 3. 이미 합의된 핵심 의사결정 (배경)

이 sub-project는 다음 결정 위에서 동작:

| 영역 | 결정 |
|------|------|
| 백엔드 언어 | Rust |
| 프론트엔드 | Next.js 16 + React 19 + TypeScript |
| DB | PostgreSQL 17 + PostGIS |
| 지도 | Naver Maps |
| 시장 | 한국만 |
| 매매 모델 | 정보 + 연락처 (옵션 A). 메시징은 Phase 2 |
| 사용자 역할 | 매수자 / 매도자 / 중개사 / 시행사 / 기업 |
| 등록 정책 | 사업자등록번호 검증 + 중개사 자격 식별 |
| 차별점 | 산업용 특화 + 공공 데이터 통합 + 분석 |
| 도메인 범위 | 풀 도메인 (매물 + 제조업체 + 분석) |
| 수익 | 광고 + 구독 (확장: 등록비, 데이터 판매) |
| 디바이스 | 반응형 웹 + PWA. 추후 네이티브 앱 |
| 품질 | SSS, 시간 무관, 비용은 돈만 |
| AI 생성 | 없음 (옵션 A). 임베딩(검색 의미 매칭)은 허용 |
| 코드 스타일 | **Biome v2.4 단독** (ESLint+Prettier 미사용) |
| 인증 IdP | **Zitadel** (Go, 가벼움, API-first, multi-tenancy 1급) |
| 라이선스 | LICENSE 없음 또는 한 줄 (사내 비공개) |
| GitHub 레포 | `gongzzang3` (변경 가능) |

---

## 4. SSS의 정의 — 7 기둥

`docs/sss-charter.md`의 SSOT 요약. 상세 정의는 그 문서.

| # | 기둥 | 의미 |
|---|------|------|
| 1 | **일관성** (Consistency) | 같은 일은 같은 방식으로. 예외 0 |
| 2 | **자동 강제** (Enforcement) | 규칙은 사람이 아니라 시스템이 강제 |
| 3 | **추적성** (Traceability) | 모든 변경·요청·결정 재구성 가능 |
| 4 | **안전성** (Safety) | 런타임 에러를 컴파일 시점에 차단 |
| 5 | **가시성** (Observability) | 서비스 상태 실시간 인지 |
| 6 | **단일 출처** (SSOT) | 한 정보 = 한 곳에만 |
| 7 | **명확성** (Clarity) | 컨벤션·네이밍으로 추측 제거 |

각 기둥은 sub-project 1에서 *방향과 도구*를 정의하고, sub-project 2+에서 *구현·운영*으로 검증된다.

---

## 5. 트리 구조 (참조 규칙 + 결과물 위치)

```
gongzzang/
├── AGENTS.md                          ← SSOT 라우터 (오픈 표준)
├── CLAUDE.md                          ← "Read @AGENTS.md" 1줄
├── README.md, TECH.md, MEMORY.md
├── .mcp.json, .gitignore, .editorconfig
│
├── .agents/                           에이전트 공용 (subagent 등)
├── .claude/                           Claude Code 전용
│   ├── settings.json
│   └── hooks/
│
├── .github/
│   ├── workflows/ci.yml
│   ├── CODEOWNERS
│   └── pull_request_template.md
│
├── docs/                              ← 도메인 SSOT 트리
│   ├── README.md
│   ├── sss-charter.md
│   ├── glossary.md
│   ├── ssot-matrix.md
│   │
│   ├── adr/                           (숫자 prefix: 시간 순서가 의미)
│   │   ├── README.md
│   │   ├── 0001-language-rust-ts.md
│   │   ├── 0002-monorepo-cargo-pnpm-turbo.md
│   │   ├── 0003-frontend-nextjs-react19.md
│   │   ├── 0004-db-postgres-postgis.md
│   │   ├── 0005-auth-zitadel.md
│   │   ├── 0006-api-rest-openapi.md
│   │   ├── 0007-cache-moka-valkey.md
│   │   ├── 0008-observability-grafana-otel-sentry.md
│   │   ├── 0009-iac-pulumi.md
│   │   ├── 0010-scope-information-platform-option-a.md
│   │   └── 0011-embedding-gemini-pgvector.md
│   │
│   ├── conventions/                   (의미명, 학습 순서는 README)
│   │   ├── README.md
│   │   ├── rust.md
│   │   ├── typescript.md
│   │   ├── sql.md
│   │   ├── naming-and-ids.md
│   │   ├── error-format.md            (RFC 9457)
│   │   ├── ui-writing-korean.md       (해요체)
│   │   ├── testing.md
│   │   ├── git-and-pr.md
│   │   └── comments.md
│   │
│   ├── infrastructure/README.md       (스켈레톤만, sub-project 8에서 채움)
│   ├── auth/README.md                 (스켈레톤만, sub-project 3)
│   ├── data/README.md                 (스켈레톤만, sub-project 2)
│   ├── cache-messaging/README.md
│   ├── backend/README.md
│   ├── api/README.md
│   ├── observability/README.md
│   ├── security/README.md
│   ├── testing/README.md
│   ├── frontend/README.md
│   ├── governance/README.md
│   ├── compliance/README.md
│   ├── cost/README.md
│   │
│   └── data-sources/
│       ├── README.md
│       ├── v-world.md
│       ├── data-go-kr.md
│       ├── korean-law.md
│       ├── nice-identity.md
│       └── naver-maps.md
│
├── memory/                            (기존 자동 메모리 유지)
│
├── apps/
│   ├── platform-web/README.md         스켈레톤
│   └── admin-web/README.md
│
├── services/
│   ├── api/README.md
│   ├── worker/README.md
│   └── data-pipeline/README.md
│
├── crates/
│   ├── domain/{core,market,regulation,insights,shared-kernel}/README.md
│   ├── data-clients/README.md
│   ├── db/README.md
│   ├── geo/README.md
│   ├── auth/README.md
│   ├── cache/README.md
│   ├── observability/README.md
│   ├── circuit-breaker/README.md
│   ├── api-types/README.md
│   ├── audit/README.md
│   └── embedding/README.md            (Phase 3 자리)
│
├── packages/
│   ├── ui-web/README.md
│   ├── api-client/README.md
│   ├── shared/README.md
│   ├── map/README.md
│   └── tsconfig/README.md
│
├── infrastructure/README.md            (스켈레톤만, sub-project 8)
├── tools/README.md
├── reference/README.md
├── tests/{e2e,load,chaos,contract}/README.md
├── db/migration/README.md
│
├── Cargo.toml                         workspace
├── rust-toolchain.toml
├── clippy.toml, rustfmt.toml, deny.toml
├── package.json                       pnpm root
├── pnpm-workspace.yaml
├── turbo.json
├── tsconfig.base.json
├── biome.json (또는 eslint+prettier)
├── lefthook.yml                       pre-commit/push hooks
├── .gitleaks.toml
├── markdownlint.json
├── renovate.json
└── .env.example
```

---

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

## 9. 검증 기준 (Sub-project 1 완료 판정)

다음 모두 YES일 때 sub-project 1 완료:

### 9.1 결과물 검증

- [ ] 60-80개 파일 모두 작성 + 커밋
- [ ] 모든 파일 ≤500줄 (자동 검증 통과)
- [ ] 모든 docs/{category}/ 에 README 존재
- [ ] 11개 ADR 모두 *컨텍스트 / 결정 / 대안 / 결과* 섹션 존재
- [ ] 9개 컨벤션 .md 모두 도구·룰·예시 포함

### 9.2 SSS 검증 (15 질문 중 가능 항목)

- [ ] (Q1) 새 외부 API 추가를 위한 표준 패턴 문서가 `docs/data-sources/README.md` + `docs/backend/circuit-breaker.md`에 존재
- [ ] (Q3) 에러 형식 SSOT (`docs/conventions/error-format.md`) 존재 + RFC 9457 명시
- [ ] (Q4) 의존성 방향 룰 정의 (`Cargo.toml` 공유 lints + dependency-cruiser 설정)
- [ ] (Q5) 시크릿 스캔 pre-commit + CI에 셋업 (gitleaks)
- [ ] (Q6) 모든 결정에 ADR 존재 (11개)
- [ ] (Q9) 코드 스타일 위반이 commit 단계에서 차단 (lefthook 검증)
- [ ] (Q11) 정보별 SSOT 매트릭스 존재 (`docs/ssot-matrix.md`)
- [ ] (Q13) 도메인 용어 사전 존재 + 위반 검증 룰 정의
- [ ] (Q14) 모든 파일 ≤500줄 자동 검증 셋업

### 9.3 작동 검증

- [ ] `pnpm install` 성공
- [ ] `cargo check` 성공 (Cargo workspace 유효)
- [ ] `pnpm turbo run lint --dry` 성공 (turbo 설정 유효)
- [ ] CI 워크플로우 PR에서 그린
- [ ] pre-commit 훅 실제 동작 (시크릿 커밋 시도 시 차단)
- [ ] AGENTS.md 라우팅 표가 모든 docs/* 위치를 정확히 가리킴 (markdown-link-check 통과)

### 9.4 사용자 검증

- [ ] 사용자가 spec 검토 후 승인
- [ ] 사용자가 결과물(60-80개 파일) 검토 후 승인

---

## 10. 의존성 + 전제 (Prerequisites)

### 10.1 로컬 환경

- Rust 1.83+ (rustup)
- Node.js 20.18+ + pnpm 9.12+
- Git 2.40+
- Docker Desktop (Phase 0 로컬 DB 셋업)

### 10.2 외부

- GitHub 레포 (이미 존재: `gongzzang3` — 또는 새로 만들지 결정 필요)
- AWS 계정 (Phase 1엔 IAM 셋업만, 인프라는 sub-project 8)
- 도메인 (TBD, sub-project 8에서 결정)

### 10.3 사용자 결정 (해결됨)

- ✅ GitHub 레포: `gongzzang3` (이름 변경 가능, 5분 작업)
- ✅ 라이선스: LICENSE 파일 없음 또는 한 줄 (`Copyright © 2026 공짱. All Rights Reserved.`). 외부 deps는 `deny.toml`로 자동 검증
- ✅ 코드 스타일: Biome v2.4 단독
- ✅ 인증 IdP: Zitadel
- [ ] CODEOWNERS의 초기 멤버 (sub-project 1 시작 시 1인부터)

---

## 11. 후속 Sub-projects (의존 그래프)

```
SP1 (헌법+모노레포)  ← 현재
 ↓
 ├─▶ SP2 (DB + Core 도메인)
 │   ↓
 │   ├─▶ SP3 (인증)
 │   ├─▶ SP4 (V-World 통합)
 │   ├─▶ SP9 (ETL 파이프라인)
 │   └─▶ SP5 (첫 API endpoint)
 │       ↓
 │       └─▶ SP6 (첫 프론트엔드)
 │           └─▶ SP11 (검색)
 │
 ├─▶ SP7 (관측성) — 병렬 가능
 ├─▶ SP8 (인프라 IaC)
 └─▶ SP12 (컴플라이언스) — 병렬 가능
```

각 sub-project는 별도 brainstorm → spec → plan → 구현 사이클.

---

## 12. 위험 + 완화

| 위험 | 영향 | 완화 |
|------|------|------|
| ADR 결정 미숙 (특히 Auth) | sub-project 3 시점 큰 재작업 | ADR-0005에 옵션·기준만, 실제 결정은 sub-project 3 brainstorm |
| 컨벤션이 처음부터 너무 엄격 | 개발 속도 저하 | Phase 1엔 *코드 0줄*이라 영향 낮음, Phase 2 시점에 조정 가능 |
| 60-80 파일 한 번에 작성 → 일관성 깨짐 | 1주차 재작업 | Implementation plan에서 작성 *순서* 정의, 의존 그래프 따라 배치 |
| 트리 구조 변경 시 모든 링크 깨짐 | 큰 재작업 | markdown-link-check CI로 즉시 발견 |
| Claude 자동 import (`@AGENTS.md`)가 실패 | 컨텍스트 누락 | Markdown 링크 병행, 모든 도구 읽기 가능하게 |

---

## 13. 자체 검토 체크리스트 (이 spec 자체)

### Placeholder 스캔
- [ ] "TBD" 또는 "TODO" 또는 미완성 섹션 없음
  - § 10.3 GitHub 레포 이름·라이선스·CODEOWNERS 멤버 = 사용자 결정 대기 (TBD 명시)
  - 이 외엔 결정 보류는 § 2.3 에 명시적으로 기록함

### 내부 일관성
- [ ] § 6 결과물 목록과 § 5 트리 구조가 일치 (스폿 체크 통과)
- [ ] § 7 AGENTS.md 룰이 § 5 의 docs 트리와 일치
- [ ] § 8 자동 강제 도구가 § 6.8 결과물과 일치

### 스코프
- [ ] Sub-project 1은 *문서 + 설정*만, 코드 0줄 — § 2.1·2.2 명확히 분리

### 모호성
- [ ] "충분히 명확한가?" — § 9 검증 기준이 객관적 측정 가능 (체크박스 형태)

---

## 14. 다음 단계

이 spec이 사용자 승인되면:

1. **writing-plans 스킬 호출** — 60-80 파일 작성 *순서·의존*을 implementation plan으로 분해
2. **executing-plans 또는 단계별 구현** — plan 따라 파일 생성
3. **검증 + 사용자 검토** — § 9 기준 통과 확인

---

## 15. 참고 자료

- 7 기둥 SSS 정의: 본 spec § 4 + (작성 예정) `docs/sss-charter.md`
- 트리 구조 영감: `daangn/seed-design`, `vercel-labs/claude-managed-agents-starter`
- AGENTS.md 오픈 표준: https://agents.md
- ADR 템플릿: MADR (https://adr.github.io/madr/)
- Conventional Commits: https://www.conventionalcommits.org
- RFC 9457 Problem Details: https://www.rfc-editor.org/rfc/rfc9457

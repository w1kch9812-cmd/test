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

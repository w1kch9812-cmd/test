# Sub-project 1 - Part 02B: SSS Charter, ADRs, And Conventions

Parent index: [Sub-project 1 Part 02](./2026-05-01-sub-project-1-charter-monorepo.part-02.md).

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

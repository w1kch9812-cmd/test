# Sub-project 1 - Part 02C: Data Sources, Domain READMEs, Workspace READMEs, And Agent Config

Parent index: [Sub-project 1 Part 02](./2026-05-01-sub-project-1-charter-monorepo.part-02.md).

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

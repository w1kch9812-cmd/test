# AGENTS.md

AI 에이전트(Claude Code / Cursor / Codex / Gemini / Cline / Aider 등) 공용 라우터.
사용자 대화·이전 컨텍스트보다 **이 파일과 @참조 문서**가 우선합니다.

---

## 0. SSS 7 기둥 헌법 (요약)

이 프로젝트는 *하이엔드 엔터프라이즈 SSS급 산업용 부동산 정보 플랫폼*입니다.
모든 작업은 다음 7 기둥을 *시스템적으로* 만족시켜야 합니다:

1. **일관성** — 같은 일은 같은 방식으로. 예외 0
2. **자동 강제** — 규칙은 사람이 아니라 시스템이 차단
3. **추적성** — 모든 변경·요청·결정 재구성 가능
4. **안전성** — 런타임 에러를 컴파일 시점에 차단
5. **가시성** — 서비스 상태 실시간 인지
6. **단일 출처(SSOT)** — 한 정보 = 한 곳에만
7. **명확성** — 컨벤션·네이밍으로 추측 제거

상세: → [docs/sss-charter.md](./docs/sss-charter.md) (작성 예정)

---

## 0.5. Cross-Repo 아키텍처 (γ' Three-Service)

이 repo 는 **3개 sibling repo 중 하나**입니다. 산단·필지·건물·제조사 마스터 데이터와
내부 직원(Staff) 신원은 별도 Rust 서비스인 `platform-core` 가 single source 가 될 예정
(M3.2 cutover).

| Repo | 도메인 | 위치 |
|---|---|---|
| 이 repo (`gongzzang3`) | B2C 부동산 플랫폼 (`gongzzang.com`) | `c:/Users/admin/Desktop/gongzzang3` |
| `platform-core` | Catalog + Workforce/Authz Hub (Rust) | `c:/Users/admin/Desktop/platform-core` |
| `seanal-sms` | B2B 산단 관리 어드민 (`saenal.com`) | `c:/Users/admin/Desktop/Seanal Site Management System` |

**의사결정 SSOT** (이 repo 의 ADR):
- [ADR 0030](./docs/adr/0030-three-service-architecture.md) — γ' 채택
- [ADR 0031](./docs/adr/0031-platform-core-bounded-contexts.md) — Catalog/Workforce 경계
- [ADR 0032](./docs/adr/0032-eventual-consistency-strategy.md) — 일관성 전략
- [ADR 0033](./docs/adr/0033-seven-guardrails-enforcement.md) — 7 Guardrails 강제
- [ADR 0034](./docs/adr/0034-catalog-ownership-handover-to-platform-core.md) — 이 repo 의 catalog 자산이 언제·어떻게 platform-core 로 이양되는지

**Sequencing SSOT**: `../platform-core/docs/migration/2026-05-11-platform-core-extraction.md` (M1~M3 단계별).

### 이 repo 작업자가 알아야 할 영향

`crates/domain/core/{industrial-complex, parcel, building, manufacturer}` 와
`crates/data-clients/{vworld, data-go-kr}`, `crates/data-pipeline-control` 은
**M3.2 cutover 시 platform-core 로 이양 예정**. 신규 write 경로 추가나 ETL 로직
변경 시 ADR 0034 의 "M1~Post-M3 상태 매트릭스" 표를 먼저 확인하세요.

B2C 도메인 (`crates/domain/core/{listing, listing-photo, user}`,
`crates/domain/market/*`, `crates/domain/insights/*`) 은 이 repo 가 영구 owner 이며
영향 없음.

---

## 1. 절대 규칙

- ❌ 모든 파일 **1500줄 초과 금지** (≤500 권장). 초과 시 폴더로 분해
- ❌ [docs/glossary.md](./docs/glossary.md) 외 도메인 용어 사용 금지
- ❌ 사용자에게 노출되는 텍스트를 LLM이 생성하지 말 것 (옵션 A 위반)
- ❌ 임시방편 코드 (`TEMP`, `HACK`, `XXX`, `ALLOWED_FOR_FRONTEND_TEMP` 류) 금지
- ❌ 메인 시스템(`apps/`, `services/`, `crates/`, `packages/`)에 MCP/LLM SDK 의존성 금지
- ❌ Pulumi 외 AWS 콘솔 직접 변경 금지 (인프라는 코드로만)
- ❌ API 키 하드코딩 / `.env` 커밋 — gitleaks가 차단
- ❌ SRID 미지정 공간 쿼리 (PostGIS 호출 시 항상 EPSG 명시)

---

## 2. 작업별 진입점 (라우팅)

| 작업 유형 | 우선 참조 |
|---------|----------|
| 새 기능 추가 | [docs/backend/](./docs/backend/README.md) + [docs/conventions/](./docs/conventions/README.md) |
| 새 외부 API 통합 | [docs/data-sources/](./docs/data-sources/README.md) + [docs/backend/circuit-breaker.md](./docs/backend/) |
| DB 스키마 변경 | [docs/data/schemas.md](./docs/data/) + [docs/data/migrations.md](./docs/data/) |
| 인증/권한 작업 | [docs/auth/](./docs/auth/README.md) + [docs/conventions/error-format.md](./docs/conventions/error-format.md) |
| UI 컴포넌트 | [docs/frontend/](./docs/frontend/README.md) + [docs/conventions/ui-writing-korean.md](./docs/conventions/ui-writing-korean.md) |
| 인프라 변경 | [docs/infrastructure/](./docs/infrastructure/README.md) (Pulumi 코드로만) |
| 새 결정 필요 | [docs/adr/README.md](./docs/adr/README.md) (ADR 작성 후 코드) |
| 관측성/로깅 | [docs/observability/](./docs/observability/README.md) |
| 보안/PII | [docs/security/](./docs/security/README.md) |
| 컴플라이언스 | [docs/compliance/](./docs/compliance/README.md) |

---

## 3. 데이터 접근 규칙 (SSS 핵심)

### 메인 시스템 (사용자 트래픽 경로)
- **공식 API 직접 호출**: V-World, 법제처(open.law.go.kr), data.go.kr
- LLM/MCP 의존성 0
- 모든 외부 호출에 Circuit Breaker + Retry + Timeout + Audit log
- raw 응답 보존 (`raw_response JSONB` 컬럼)

### AI 에이전트 경로 (개발자 Claude 세션 한정)
- MCP 사용 가능 (개발/탐색용)
- 메인 코드에 import 금지

### 향후 옵션 C (AI 어시스턴트, 별도 모듈)
- `apps/ai-assistant/` 자리만 비워둠
- 도입 시 verify_citations 등 환각 방지 의무

---

## 4. 자동 강제 흐름

```
1. 에디터        rust-analyzer + Biome 확장        실시간 lint/format
2. pre-commit    lefthook + gitleaks               format + 빠른 lint + 시크릿 스캔 + 파일 크기
3. pre-push      lefthook                          typecheck + cargo check/clippy + 링크 체크
4. CI (PR)       GitHub Actions                    풀스택 (lint/type/test/SAST/SCA/cargo-deny/SBOM)
5. CI (merge)    GitHub Actions                    이미지 빌드 + 서명 + 배포
```

---

## 5. 한국어 규칙

- 사용자 노출 문자열: **해요체** (예: "조회했어요", "잠시 후 다시 시도해 주세요")
- 에러 메시지: **원인 + 대응 안내**
- 법령 인용: 정식 명칭 + 조·항·호 (예: "국토의 계획 및 이용에 관한 법률 제76조제5항")
- 도메인 용어: [docs/glossary.md](./docs/glossary.md) 의 영문 식별자 사용 (코드)
- 로그/커밋: 영어 (Conventional Commits)

---

## 6. 사용자 확인 필요한 작업

- 새 npm/cargo 패키지 추가
- DB 스키마 변경 (마이그레이션 생성 전 승인)
- 인증/권한/개인정보 로직 수정
- V-World 쿼터에 영향을 줄 배치 작업
- 공공데이터 재배포/오픈소스 공개
- `git push --force`, `git reset --hard`, 브랜치 삭제

---

## 7. 도메인 어휘 (요약, 상세는 glossary)

| 한국어 | 영문 (코드) |
|--------|------------|
| 필지 | `Parcel` |
| 매물 | `Listing` |
| 사업자등록번호 | `BusinessNumber` |
| 공인중개사 | `Broker` |
| 산업단지 | `IndustrialComplex` |
| 지식산업센터 | `KnowledgeIndustryCenter` |
| 실거래가 | `RealTransactionPrice` |
| 공시지가 | `OfficialLandPrice` |
| 시행사 | `Developer` |
| 매도자 | `Seller` |
| 매수자 | `Buyer` |

전체: → [docs/glossary.md](./docs/glossary.md)

---

## 8. SSOT 원칙

각 정보는 **한 곳에만** 존재. 사본이 있으면 그것이 사본임을 명시.

- 사용자 데이터 → PostgreSQL `user` (Redis 세션은 사본)
- 공공 API raw → DB `raw_response JSONB` (캐시는 사본)
- 비즈니스 규칙 → `crates/domain/*` Rust 코드
- API 계약 → Rust 코드 + utoipa (OpenAPI 자동, TS 타입 자동)
- DB 스키마 → `db/migration/V*.sql` (sqlx가 자동 검증)
- 인프라 → Pulumi TypeScript (AWS 콘솔 수동 변경 금지)
- 도메인 용어 → [docs/glossary.md](./docs/glossary.md)

상세 매트릭스: → [docs/ssot-matrix.md](./docs/ssot-matrix.md)

---

## 9. 1500줄 안티패턴 경보

`docs/schema.md` 1349줄, `docs/site-builder.md` 1447줄 같은 거대 SSOT 파일 = **이름만 SSOT**.
폴더 단위 SSOT가 진짜 SSOT.

- 500줄 도달 → 분해 검토
- 1500줄 도달 → CI가 차단
- *처음부터* 폴더로 시작

---

## 10. SSS-grade Panel System Axes

패널 시스템은 URL-driven enterprise interaction surface다. 모든 panel 변경은 아래 축을 만족해야 한다. (Claude + Codex 합의, 2026-05-08)

### 10.1 Day-1 BLOCKER (없으면 SSS 자격 박탈)

1. **Correctness**
   - URL serialize/deserialize roundtrip 100%
   - reload / back / forward / mobile back 동작 100%
   - hydration mismatch, race leak, memory leak 0

2. **Accessibility**
   - WCAG 2.2 AA 기준
   - keyboard-only 주요 flow 100%
   - dialog / focus / ESC / breadcrumb은 ARIA APG 패턴 준수
   - axe violation 0 in CI

3. **Type Safety**
   - TS strict + discriminated union
   - panel kind / view exhaustiveness compile-time enforced
   - API 계약은 Rust → utoipa → OpenAPI → generated TS only

4. **SSOT**
   - URL = panel state SSOT
   - registry = kind / view / component / fetch / i18n / telemetry SSOT
   - panel framework는 kind implementation을 import 금지
   - ad-hoc URL parsing 금지 — codec만 허용

5. **Security & Privacy**
   - user-facing string은 typed i18n only
   - PII log / span / event 금지
   - CSP / XSS / CSRF / rate-limit baseline 유지
   - audit-relevant panel/API actions는 correlation_id로 추적 가능해야 함

6. **Migration / Versioning**
   - 한 번 배포된 URL codec은 영구 backward-compatible
   - invalid / unknown URL은 safe recovery + telemetry
   - codec 변경은 ADR + compatibility corpus test 필수

### 10.2 Day-1 MUST

7. **Resilience** — per-panel error boundary, AbortController / query cancellation, loading / error / empty / auth-required / ok state 강제
8. **Observability** — `panel.opened` / `panel.url_decode_failed` / fetch latency span 필수, telemetry schema test 100%, panel open latency SLO 측정 가능해야 함
9. **Performance** — LCP < 2.5s p75, INP < 200ms p75, CLS < 0.1 p75, bundle budget CI gate
10. **Governance** — panel architecture 변경은 ADR 필요, lefthook + CI로 URL SSOT / codec / import boundary 강제

### 10.3 Phase-2 Hardening

11. **Contract Testing** — OpenAPI breaking change diff, generated client compile gate, no-mock integration tests for backing endpoints
12. **Supply Chain Integrity** — CycloneDX SBOM, cargo-deny / pnpm audit / gitleaks, signed artifacts
13. **Operations** — readiness / health checks, feature flag 및 rollback path, SLO dashboard + runbook + alert policy
14. **Data Lineage** — `raw_response` 보존, source / fetched_at / SRID / license traceable, schema evolution policy documented
15. **Design System / Documentation** — Spec → ADR → Code traceability, Storybook + visual regression (critical states only), C4 recommended *not* CI gate

### 10.4 명시적 비포함 (SSS 라벨에 본질 아님)

- 모든 페이지 visual regression — critical states (panel shell / mobile fullscreen / side-by-side / 4-state) 만
- Unit 100% branch coverage — 핵심 순수 로직(codec / URL parser / permission / calculation)만 100%, UI는 risk-based
- Mutation testing 전체 적용 — 핵심 순수 로직에만 selective
- Property-based testing 전체 — codec / SRID / idPattern 등 selective
- Offline support — 산업용 부동산 조회에는 read-through cache로 충분
- Chaos engineering Day-1 — Phase-2 hardening에서 검토
- C4 diagram CI blocker — 문서 형식주의 위험

### 10.5 적용 범위

본 §10은 *패널 시스템* 한정 SSS 정의이며, 다른 도메인(auth, infra, listings backend 등)은 자체 SSS axis가 필요할 수 있다. §10의 BLOCKER 항목 중 *Type Safety / SSOT / Security & Privacy / Migration* 은 도메인 무관 일반 룰이므로 다른 영역에도 동일하게 적용한다.

참조 표준: W3C WCAG 2.2 AA, W3C ARIA APG, Google Core Web Vitals, OWASP ASVS, NIST SSDF SP 800-218, OpenTelemetry Semantic Conventions, CycloneDX SBOM, PIPC / 개인정보보호법.

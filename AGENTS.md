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

# TECH.md

기술 스택 + SSOT 맵. 세부 내용은 각 링크 참조.

> **프로젝트**: 산업용 부동산 정보 플랫폼 (옵션 A — 데이터 플랫폼).
> **AI 생성 텍스트 노출 없음**. 임베딩(검색 의미 매칭, Phase 3+)은 허용.
> **시장**: 한국만. **품질**: SSS 엔터프라이즈, 시간 무관, 비용은 돈만 고려.

---

## 1. 기술 스택 (확정)

| 영역 | 선택 | 결정 근거 |
|------|------|---------|
| 백엔드 언어 | **Rust 1.83+** | → [docs/adr/0001-language-rust-ts.md](./docs/adr/0001-language-rust-ts.md) |
| 프론트엔드 | **Next.js 16 + React 19 + TypeScript 5.7** | → [docs/adr/0003-frontend-nextjs-react19.md](./docs/adr/0003-frontend-nextjs-react19.md) |
| DB | **PostgreSQL 17 + PostGIS** | → [docs/adr/0004-db-postgres-postgis.md](./docs/adr/0004-db-postgres-postgis.md) |
| HTTP 서버 | **Axum** + tokio | ADR-0001 |
| ORM/SQL | **SQLx** (compile-time SQL 검증) | ADR-0001 |
| 지도 | **Naver Maps SDK** (한국어/한국 시장) | ADR-0003 |
| 인증 IdP | **Zitadel** (Go, API-first, 가벼움) | → [docs/adr/0005-auth-zitadel.md](./docs/adr/0005-auth-zitadel.md) |
| API 계약 | **REST + OpenAPI 3.1** (utoipa + openapi-typescript) | → [docs/adr/0006-api-rest-openapi.md](./docs/adr/0006-api-rest-openapi.md) |
| 캐시 | **moka (L1) + Valkey (L2)** | → [docs/adr/0007-cache-moka-valkey.md](./docs/adr/0007-cache-moka-valkey.md) |
| 검색 | **PostgreSQL FTS** (Phase 1) → **Meilisearch** (Phase 3) | (작성 예정) |
| 임베딩 | **Gemini Embedding 2 + pgvector** (Phase 3+) | → [docs/adr/0011-embedding-gemini-pgvector.md](./docs/adr/0011-embedding-gemini-pgvector.md) |
| 모노레포 | **Cargo workspace + pnpm + Turborepo** | → [docs/adr/0002-monorepo-cargo-pnpm-turbo.md](./docs/adr/0002-monorepo-cargo-pnpm-turbo.md) |
| 코드 스타일 | **Biome v2.4** (단독, 보안은 Semgrep+Snyk+gitleaks) | (작성 예정) |
| IaC | **Pulumi (TypeScript)** | → [docs/adr/0009-iac-pulumi.md](./docs/adr/0009-iac-pulumi.md) |
| 관측성 | **Grafana + Prometheus + Loki + Tempo + Sentry + OTel** | → [docs/adr/0008-observability-grafana-otel-sentry.md](./docs/adr/0008-observability-grafana-otel-sentry.md) |
| 본인인증 | **NICE 평가정보** (도입 시점은 sub-project 3) | → [docs/data-sources/nice-identity.md](./docs/data-sources/nice-identity.md) |

---

## 2. 데이터 소스 (SSOT)

프로덕션은 공식 API 직접, 에이전트는 MCP 별도. 자세한 규칙은 [AGENTS.md §3](./AGENTS.md).

| 소스 | 운영 기관 | 진입점 | 문서 |
|------|----------|-------|------|
| V-World | 공간정보산업진흥원 | api.vworld.kr (REST/WMS/WFS) | [docs/data-sources/v-world.md](./docs/data-sources/v-world.md) |
| 법제처 | 법제처 | open.law.go.kr Open API | [docs/data-sources/korean-law.md](./docs/data-sources/korean-law.md) |
| 공공데이터포털 | 행정안전부 | data.go.kr REST | [docs/data-sources/data-go-kr.md](./docs/data-sources/data-go-kr.md) |
| NICE 본인인증 | NICE 평가정보 | (CP 등록 후) | [docs/data-sources/nice-identity.md](./docs/data-sources/nice-identity.md) |
| Naver Maps | 네이버 클라우드 | maps.apigw.ntruss.com | [docs/data-sources/naver-maps.md](./docs/data-sources/naver-maps.md) |

---

## 3. 좌표계 (SRID)

| SRID | 용도 | 비고 |
|------|------|------|
| 4326 | WGS84 경위도 | **저장·입출력 표준** |
| 5179 | UTM-K | V-World 기본, 거리 연산 |
| 5186 | 중부원점 TM | 지적도 |
| 3857 | Web Mercator | 타일 렌더 |

**규칙**: DB 저장은 4326, 거리 연산은 5179, 타일은 3857. 변환 누락 차단 lint 룰 (sub-project 4+).

---

## 4. 모노레포 구조 (확정)

```
gongzzang/
├── apps/                       TS / Next.js (UI만, 비즈니스 로직 0)
│   ├── platform-web/           메인 사용자 사이트 + PWA
│   └── admin-web/              관리자 대시보드
│
├── services/                   Rust 실행 가능 바이너리
│   ├── api/                    HTTP API 서버 (Axum)
│   ├── worker/                 배치/크론 (Tokio + advisory lock)
│   └── data-pipeline/          ETL (공공 API → DB)
│
├── crates/                     Rust 라이브러리 (DDD)
│   ├── domain/                 4 BC: core / market / regulation / insights / shared-kernel
│   ├── data-clients/           외부 공공 API HTTP 클라이언트 + ACL
│   ├── db/                     SQLx + PostGIS Repository
│   ├── geo/                    좌표 변환 (Naver 호환)
│   ├── auth/                   Zitadel JWT 검증, RBAC
│   ├── cache/                  moka L1 + Valkey L2
│   ├── observability/          tracing + OTel + Sentry
│   ├── circuit-breaker/        외부 호출 표준
│   ├── api-types/              utoipa OpenAPI 노출 타입
│   ├── audit/                  immutable audit log
│   └── embedding/              Phase 3+ (Gemini + pgvector)
│
├── packages/                   TS 라이브러리 (프론트 전용)
│   ├── ui-web/                 shadcn/ui + Tailwind v4
│   ├── api-client/             OpenAPI 자동 생성 SDK
│   ├── shared/                 공용 훅, 유틸
│   ├── map/                    네이버 지도 + Canvas 마커
│   └── tsconfig/               공유 TS 설정
│
├── infrastructure/             Pulumi TypeScript (sub-project 8)
├── tools/                      빌드 스크립트, 코드젠
└── db/migration/               sqlx migrate / Flyway
```

### 의존성 방향 (강제)

```
apps/*          → packages/{ui-web, api-client, shared, map}
services/*      → crates/*
crates/domain/* → crates/{shared-kernel} 만 (다른 BC import 금지)
crates/data-clients/* → crates/{circuit-breaker, observability, api-types}
crates/db       → crates/{domain (ports만), api-types}
```

화살표 거꾸로 = CI 빌드 실패 (sub-project 5+에 dependency-cruiser/cargo-arch 도입).

---

## 5. SSOT 매트릭스 (요약)

| 정보 | 진짜 SSOT | 사본 |
|------|---------|------|
| 사용자 데이터 | PostgreSQL `user` 테이블 | Redis 세션 |
| 공공 API raw | DB `raw_response JSONB` | Redis 캐시 |
| 비즈니스 규칙 | `crates/domain/*` Rust 코드 | 테스트, 문서 |
| API 계약 | Rust + utoipa | `openapi.json` (자동), TS 타입 (자동) |
| DB 스키마 | `db/migration/V*.sql` | Rust 타입 (sqlx 자동) |
| 인프라 | Pulumi TS 코드 | AWS 콘솔 (수동 변경 금지) |
| 도메인 용어 | `docs/glossary.md` | 모든 코드/UI/문서 |

상세: → [docs/ssot-matrix.md](./docs/ssot-matrix.md) (작성 예정)

---

## 6. Phase별 비용 추정 (AWS Seoul + Cloudflare Free)

| Phase | 사용자 | 월 비용 (RI 적용 후) |
|-------|--------|---------------------|
| Phase 0 (코드 작성) | 0 | ₩0 |
| Phase 1 (스테이징) | 0 | ~₩5만 |
| Phase 2 (베타) | 1,000 | ~₩20만 |
| Phase 3 (출시) | 10,000 | ~₩55만 (RI 30% 할인) |
| Phase 4 (성장) | 100,000 | ~₩195만 |

상세: → [docs/cost/](./docs/cost/README.md) (작성 예정)

---

## 7. 환경 변수

→ [.env.example](./.env.example) 참조. 실제 키는 AWS Secrets Manager / Vault (sub-project 8).

---

## 8. 컴플라이언스 (계획)

- **PIPA** (한국 개인정보보호법): 처음부터 PII 분류 + 마스킹
- **ISMS-P** 인증: Phase 3 후반 (매출 발생 후)
- **SOC 2** Type II: B2B 진출 시 (Phase 4+)
- **공공데이터 라이선스**: 각 데이터셋별 이용허락범위 자동 검증

상세: → [docs/compliance/](./docs/compliance/README.md) (작성 예정)

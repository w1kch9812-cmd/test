# Architecture Decision Records (ADR)

모든 기술·아키텍처 결정의 영구 기록.

## 작성 원칙

- 시간 순서가 의미 → `NNNN-` prefix 유지
- 한 결정 = 한 파일
- 승인 후 *수정 금지*. 변경은 새 ADR로 (Supersedes 표기)
- 재검토 / 보류는 *trigger 명시*

## 템플릿

```markdown
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
```

## 인덱스

| # | 제목 | 상태 |
|---|------|------|
| [0001](./0001-language-rust-ts.md) | 언어 — Rust + TypeScript | Accepted |
| [0002](./0002-monorepo-cargo-pnpm-turbo.md) | 모노레포 — Cargo + pnpm + Turborepo | Accepted |
| [0003](./0003-frontend-nextjs-react19.md) | 프론트엔드 — Next.js 16 + React 19 | Accepted |
| [0004](./0004-db-postgres-postgis.md) | DB — PostgreSQL 17 + PostGIS | Accepted |
| [0005](./0005-auth-zitadel.md) | 인증 IdP — Zitadel | Accepted |
| [0006](./0006-api-rest-openapi.md) | API — REST + OpenAPI (utoipa) | Accepted |
| [0007](./0007-cache-moka-valkey.md) | 캐시 — moka L1 + Valkey L2 | Accepted |
| [0008](./0008-observability-grafana-otel-sentry.md) | 관측성 — Grafana + OTel + Sentry | Accepted |
| [0009](./0009-iac-pulumi.md) | IaC — Pulumi (TypeScript) | Accepted |
| [0010](./0010-scope-information-platform-option-a.md) | 범위 — 산업용 부동산 정보 플랫폼 (옵션 A) | Accepted |
| [0011](./0011-embedding-gemini-pgvector.md) | 임베딩 — Gemini + pgvector (Phase 3+) | Accepted |
| [0012](./0012-pipeline-visualization-react-flow.md) | 파이프라인 시각화 — React Flow (xyflow) | Accepted |
| [0013](./0013-listing-search-naver-maps.md) | Listing 검색 지도 vendor — Naver Maps (SP6-ii) | Accepted |

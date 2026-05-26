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
| [0014](./0014-base-layer-defer-pmtiles.md) | 지도 base layer (전국 필지 polygon) — 보류 (R2 PMTiles SSS 부적합) | **Superseded by 0016** |
| [0015](./0015-v-world-acl-rearchitecture.md) | V-World ACL 재설계 — fixture-driven, layer-decomposed, envelope-aware | Accepted |
| [0016](./0016-medallion-base-layer-postgis-silver-pmtiles-gold.md) | 지도 base layer — PMTiles 100% (Bronze raw archive + Gold PMTiles, PostGIS Silver 미도입) | Accepted |
| [0017](./0017-listing-marker-render-canvas-bitmap-stamp.md) | 매물 마커 렌더링 — Naver Marker + Canvas content + BitmapStampCache (단일 렌더 박자) | Accepted |
| [0018](./0018-pnu-first-identity-no-coordinates.md) | 매물 정체성 — PNU-First (좌표는 매칭/검색에 사용 안 함) | Accepted |
| [0019](./0019-pmtiles-source-via-addsourcetype.md) | PMTiles 통합 — VectorTileSource subclass + Service Worker transport | **Superseded by 0021** |
| [0020](./0020-naver-vector-interaction-model.md) | Naver gl SDK vector 한계 + 우리 platform interaction model (probe scope = polygon-only) | Accepted |
| [0021](./0021-static-vector-tile-decomposition.md) | PMTiles 분해 → 정적 `{z}/{x}/{y}.pbf` (mapbox-gl 표준 100%, trick 0) | Accepted |
| [0022](./0022-bronze-scraping-isolated-python-service.md) | Bronze HTML scraping = 격리 Python service (`services/scraper-py/`) + Scrapling | Accepted |
| [0023](./0023-audit-2026-05-08-hardening.md) | Codex audit 2026-05-08 hardening — `/internal/auth/event` shared secret + production fail-fast + JTI fail-closed + structured map errors | Accepted (partial — handoff) |
| [0024](./0024-etl-cancel-protocol-immediate-abort.md) | ETL cancel protocol — 즉시 abort + L3 staging atomicity 보호 (state machine 거부) | Accepted |
| [0025](./0025-bronze-scraping-workflow-orchestrator-not-rust-spawn.md) | Bronze scraping orchestration — GitHub Actions workflow phase split (Rust가 Python spawn 안 함, ADR 0022 amendment) | Accepted |
| [0026](./0026-bronze-api-archive-r2-not-postgres-jsonb.md) | Bronze API archive — R2 (S3-호환 객체 저장소) 로 이전, Postgres jsonb 폐기 (cost + UPSERT 손실 + 시계열 보존) | Accepted |
| [0027](./0027-admin-complex-layer-source-deferred.md) | admin/complex layer ETL source 결정 보류 + `Layer::is_active_in_etl` SSOT gate (parcel prefix 임시 재사용 차단) | Accepted |
| [0028](./0028-supply-chain-sha-pin-and-cleanup-cron.md) | Supply-chain SHA pin; manifest cleanup portion superseded by ADR 0036/platform-core ADR 0004 | Accepted |
| [0029](./0029-explicit-environment-separation.md) | `ETL_ENVIRONMENT` 명시 분리 + secret namespace 격리 (verify smoke 사고 후속) | Accepted (Superseded in part by 0035) |
| [0030](./0030-three-service-architecture.md) | γ' Three-Service Architecture 채택 (gongzzang / platform-core / dawneer) | Accepted |
| [0031](./0031-platform-core-bounded-contexts.md) | platform-core Bounded Contexts — Catalog / Workforce 경계 | Accepted |
| [0032](./0032-eventual-consistency-strategy.md) | Cross-service Eventual Consistency 전략 | Accepted |
| [0033](./0033-seven-guardrails-enforcement.md) | 7 Guardrails — cross-service 코드 리뷰 자동 강제 | Accepted |
| [0034](./0034-catalog-ownership-handover-to-platform-core.md) | Catalog 자산 platform-core 로 이양 (M3.2 cutover) | Accepted |
| [0035](./0035-legacy-r2-removal-and-atomic-namespace.md) | Legacy `R2_*` + `ETL_BUILD_ENV` 완전 제거 + atomic namespace 강제 (ADR 0029 backward-compat 제거) | Accepted |
| [0036](./0036-static-vector-tile-runtime-contract.md) | Static vector tile runtime contract - platform-core-owned manifest, Gongzzang consumer only | Accepted |
| [0037](./0037-pnu-anchor-pbf-marker-tiles.md) | PNU Anchor PBF Marker Tiles - platform-core anchor positions, Gongzzang PBF marker runtime | Accepted |
| [0038](./0038-listing-marker-serving-index-filter-mask.md) | Listing Marker Serving Index And Filter Mask - scalable dynamic listing marker/filter serving | Accepted |

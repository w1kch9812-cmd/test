# data/

DB, PostGIS, 마이그레이션, 데이터 거버넌스 SSOT.

## 책임 영역
- PostgreSQL 17 + PostGIS 3.5+
- pgvector (Phase 3+ 임베딩)
- SQLx + 마이그레이션
- 데이터 모델 (DDD aggregate)
- 좌표계 변환 (4326 / 5179 / 5186 / 3857)
- 데이터 카탈로그 (OpenMetadata, Phase 3+)
- 데이터 품질 (Soda 또는 Great Expectations, Phase 3+)
- CDC (Debezium, Phase 3+)
- 백업 / PITR (pgBackRest)
- retention 정책

## 작성 예정 문서 (sub-project 2)
- `postgres.md` — DB 설정, 인스턴스 사이즈
- `postgis.md` — 공간 쿼리 패턴, 인덱스 전략
- `medallion.md` — Gongzzang-owned data only; Catalog Bronze/Silver/Gold 는 Platform Core 소유
- `schemas.md` — 도메인별 스키마 분리
- `migrations.md` — sqlx migrate / Atlas
- `search.md` — Postgres FTS (Phase 1) → Meilisearch (Phase 3)
- `embedding.md` — pgvector (Phase 3+, ADR-0011)
- `retention.md` — audit/raw 보존 기간 정책
- `backup.md` — pgBackRest + S3
- `cdc.md` — Debezium (Phase 3+)

## 관련 ADR
- → @docs/adr/0004-db-postgres-postgis.md
- → @docs/adr/0011-embedding-gemini-pgvector.md

## 관련 컨벤션
- → @docs/conventions/sql.md

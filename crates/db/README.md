# crates/db

PostgreSQL + PostGIS 어댑터 (Repository 구현).

## 책임
- SQLx 클라이언트 팩토리 (connection pool)
- 도메인 Repository trait 구현 (각 Aggregate별)
- PostGIS 공간 쿼리 헬퍼
- Optimistic Locking (version 컬럼)
- raw_response JSONB 저장
- 마이그레이션은 `db/migration/` (sqlx migrate)

## 의존
- `crates/domain/*` (trait만)
- `crates/api-types` (에러 타입)
- `crates/observability`
- `sqlx` (postgres + macros + chrono + uuid)
- `postgis` crate (geometry 타입)

## 정책
- compile-time SQL 검증 (`sqlx::query!` / `sqlx::query_as!`)
- 모든 geometry 컬럼 SRID 명시 (4326)
- Optimistic locking 위반 = 명시적 에러 (`OptimisticLockConflict`)
- 트랜잭션 = Repository 메서드 단위
- Cross-aggregate transaction 금지 (도메인 이벤트 + Outbox)

→ ADR-0004, → @docs/conventions/sql.md

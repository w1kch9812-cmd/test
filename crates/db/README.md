# crates/db

`SQLx` `Postgres` `Repository` 구현체에요.

## 책임

- 도메인 BC가 정의한 `*Repository` trait의 `Postgres` 구현이에요.
- `Optimistic Locking` (`version` 컬럼) 강제해요.
- `SQL` 에러는 `RepoError::Database`로 매핑 (메시지만 — 정보 누설 방지).

## 범위

- **Walking Skeleton**: `PgUserRepository`만 (`find_by_id` + `save`).
- **Sub-project 5**: `Listing`, `ListingPhoto` 등 모든 `Aggregate` `Repository` 추가.

## 의존

- `crates/domain/core/user` (`UserRepository` trait)
- `crates/shared-kernel` (`Id`, `Email` 등 값 객체)
- `sqlx` (`postgres` + `chrono`)

## 정책

- 고정된 SQL shape 는 `sqlx::query!`, `sqlx::query_as!`, `sqlx::query_scalar!`
  매크로로 작성해 컴파일 시점에 schema drift 를 차단해요.
- `.sqlx/` prepare metadata 는 쿼리와 DB schema 사이의 계약이므로 커밋 대상이에요.
- 검색 필터/정렬처럼 SQL shape 가 실제로 동적인 경우에만 runtime `sqlx::query`
  계열을 허용하고, 해당 위치에는 동적 SQL 사유와 parameter binding 보장을 남겨요.
- `unwrap`/`expect` 금지 — 모든 에러는 `RepoError`로 매핑해요.

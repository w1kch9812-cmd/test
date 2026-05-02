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

- 현재는 `sqlx::query` 런타임 `API` 사용 (compile-time `DB` 연결 불필요).
- Sub-project 5에서 `sqlx prepare` 캐시 도입 후 `query!` 매크로로 전환해요.
- `unwrap`/`expect` 금지 — 모든 에러는 `RepoError`로 매핑해요.

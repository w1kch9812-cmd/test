# user-domain

공짱 `User` Aggregate (Core BC, RDS 동적).

## Walking Skeleton 범위

- `User` struct — `id`, `zitadel_sub`, `email`, `display_name`, `user_kind`, `created_at`, `updated_at`, `version`
- `UserKind` enum — 개인 / 법인
- `User::try_new` — `display_name` (≤100자) + `zitadel_sub` (≤255자) 검증
- `UserRepository` trait — `find_by_id` / `save`
- `UserError` / `RepoError` — thiserror

## 의존

- `shared-kernel` — `Email`, `Id<UserMarker>`
- 외부 의존 0 (도메인 순수성)

## Plan 2b-i에서 추가 예정

- `business_verified`, `broker`, soft-delete (`deleted_at`)
- RBAC roles, 도메인 이벤트
- `update_*` 메서드 (Optimistic locking)

→ ADR-0001, → @docs/conventions/rust.md

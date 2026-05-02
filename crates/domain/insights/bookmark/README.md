# bookmark-domain

`Bookmark` 도메인 (Insights BC, RDS 동적) crate에요.

## 책임

- spec § 5.2 `bookmark_listing` + `bookmark_external` 두 테이블 매핑하는
  Aggregate 2종 정의해요.
- `BookmarkListing` — 매물 북마크 (composite PK `user_id + listing_id`,
  `Listing` FK + `ON DELETE CASCADE`).
- `BookmarkExternal` — 외부 R2 entity 북마크 (polymorphic
  `target_kind` + `target_id`).
- `BookmarkExternalKind` enum 4값 (`parcel`, `court_auction`,
  `manufacturer`, `industrial_complex`).
- `BookmarkRepository` trait — 구현체는 sub-project 5에서 추가.

## 의존

- `shared-kernel` (`Id`, `UserMarker`, `ListingMarker`,
  `BookmarkExternalMarker`).

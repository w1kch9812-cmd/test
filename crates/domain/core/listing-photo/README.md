# listing-photo-domain

`ListingPhoto` Aggregate (Core BC, RDS 동적 + R2 binary) crate에요.

## 책임

- spec § 5.1 `listing_photo` 테이블 12 필드 매핑하는 `ListingPhoto`
  Aggregate 정의해요.
- 메타데이터(크기, content-type, 캡션, 순서)는 RDS, 실제 바이너리는
  R2 객체로 분리 — `r2_key`/`thumbnail_r2_key`로 참조해요.
- `try_new` 생성자에서 `r2_key` 비어있음, `display_order` 음수,
  `caption` 200자 초과를 차단해요.
- `soft_delete`(idempotent), `reorder`, `is_active` 도메인 메서드 제공.
- `ListingPhotoRepository` trait — 구현체는 sub-project 5에서 추가.

## 의존

- `shared-kernel` (`Id`, `ListingMarker`, `ListingPhotoMarker`).
- `listing-domain` 의존 *없어요* — `ListingMarker`는
  `shared-kernel::id` 거쳐 phantom-typed FK로 참조해요.

## 예시

```rust,ignore
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
let photo = ListingPhoto::try_new(/* … */)?;
```

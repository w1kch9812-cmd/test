# listing-domain

`Listing` Aggregate (Core BC, RDS 동적) crate에요.

## 책임

- spec § 5.1 listing 테이블 20 필드 매핑하는 `Listing` Aggregate 정의해요.
- `try_new_draft` 생성자에서 V003_01 cross-field invariant
  (`transaction_type` ↔ `deposit`/`monthly_rent`)를 강제해요.
- 도메인 mutation 메서드 (`submit_for_review`, `approve`,
  `mark_sold` 등) + `ListingRepository` trait는 후속 task (T11)에서
  추가됩니다.

## 의존

- `shared-kernel` (Id, MoneyKrw, AreaM2, Pnu, ListingType,
  TransactionType, ListingStatus, ContactVisibility, ListingTitle,
  Description).
- `user-domain` 의존 *없어요* — `UserMarker`는 `shared-kernel::id` 거쳐요.

## 예시

```rust,ignore
use listing_domain::entity::Listing;
let listing = Listing::try_new_draft(/* … */)?;
```

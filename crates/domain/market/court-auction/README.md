# court-auction-domain

`CourtAuction` Aggregate (Market BC, R2 정적 — Reader trait only) crate에요.

## 책임

- 한국 법원 경매 공개 데이터를 ETL해 R2에 보관한 데이터를 읽어요.
  활성 + 이력 모두 포함하며 Aggregate는 *read-only* — mutation 메서드 0개.
- 한 필지(`Pnu`)에 다수 사건이 가능해요.
- `CourtAuctionReader` trait 포트 — 구현체는 sub-project 4.
- `ReaderError` enum — `NotFound` / `Fetch` / `Parse`.
- `CourtAuctionKind` (BC-internal, 강제/임의/기타).
- `CourtAuctionStatus` (BC-internal, 예정/진행중/낙찰/취하/유찰).
  `is_active()` 헬퍼는 `Upcoming` + `InProgress` 필터링용이에요.

## 의존

- `shared-kernel` (`Pnu`, `MoneyKrw`, `PointSrid`, `SpatialScope`).
- 다른 BC 의존 *없어요*.

## 예시

```rust,ignore
use court_auction_domain::reader::CourtAuctionReader;
let active = reader.fetch_active().await?;
```

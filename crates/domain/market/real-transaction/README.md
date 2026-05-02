# real-transaction-domain

`RealTransaction` Aggregate (Market BC, R2 정적 — Reader trait only) crate에요.

## 책임

- 한국 실거래가 공개 데이터(`data.go.kr`)를 ETL해 R2에 보관한 데이터를 읽어요.
  Aggregate는 *read-only* — mutation 메서드 0개.
- 한 필지(`Pnu`)에 다수 거래가 가능해요 (시간 순).
- `RealTransactionReader` trait 포트 — 구현체는 sub-project 4.
- `ReaderError` enum — `NotFound` / `Fetch` / `Parse`.
- `TransactionKind` (BC-internal, 매매/전세/월세).

## `TransactionKind` vs `shared_kernel::TransactionType`

같은 3 변형이지만 의도적으로 분리되어 있어요. `TransactionType`은
Listing(현재 매물) 용, `TransactionKind`는 RealTransaction(과거 거래) 용
이에요. 두 시스템의 진화 경로가 달라서 BC-internal로 유지해요.

## 의존

- `shared-kernel` (`Pnu`, `MoneyKrw`, `AreaM2`, `BoundingBox`).
- 다른 BC 의존 *없어요*.

## 예시

```rust,ignore
use real_transaction_domain::reader::RealTransactionReader;
let txs = reader.fetch_by_pnu(&pnu).await?;
```

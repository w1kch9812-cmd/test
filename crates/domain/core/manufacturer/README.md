# manufacturer-domain

`Manufacturer` Aggregate (Core BC, `R2` 정적 — Reader trait only) crate에요.

## 책임

- 한국 제조업체 (산단 입주기업 등) 데이터 — `KOSIS`/data.go.kr/`KICOX`에서
  ETL 후 `R2`에 보관해요.
- Aggregate 자체는 *read-only* — V1 사용자 트래픽 경로에서 변경하지
  않아요. mutation 메서드 0개.
- 식별자는 `BusinessNumber` (10자리, `R2` 정적이므로 `FK` 아님).
- `EmployeeCountBand` 6종 (`KOSIS` 인원 구간 기준).
- `ManufacturerReader` trait 포트 — 구현체는 sub-project 4
  (`crates/data-clients/r2-public-data/`)에서 추가해요.
- `ReaderError` enum — `NotFound` / `Fetch` / `Parse`.

## 의존

- `shared-kernel` (`BusinessNumber`, `Pnu`, `KsicCode`).
- 다른 BC 의존 *없어요* (`industrial_complex_code` 는 단순 `String` —
  R2 정적이므로 cross-BC FK 회피).

## 예시

```rust,ignore
use manufacturer_domain::reader::ManufacturerReader;
let m = reader.fetch_by_business_number(&bn).await?;
```

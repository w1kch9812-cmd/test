# parcel-domain

`Parcel` Aggregate (Core BC, R2 정적 — Reader trait only) crate에요.

## 책임

- spec § 8.4 `Parcel` Aggregate 10 필드 매핑해요.
- 한국 필지 (`Parcel`) 데이터 — V-World/data.go.kr에서 ETL 후 R2에 보관해요.
- Aggregate 자체는 *read-only* — V1 사용자 트래픽 경로에서 변경하지
  않아요. mutation 메서드 0개.
- `ParcelReader` trait 포트 — 구현체는 sub-project 4
  (`crates/data-clients/r2-public-data/`)에서 추가해요.
- `ParcelMarker` lightweight projection — 지도 마커 렌더용 4 필드.
- `ReaderError` enum — `NotFound` / `Fetch` / `Parse`.

## 의존

- `shared-kernel` (`Pnu`, `AdminDivision`, `RoadAddress`, `JibunAddress`,
  `LandUseType`, `AreaM2`, `MoneyKrw`, `Zoning`, `PolygonSrid`, `PointSrid`,
  `BoundingBox`).
- 다른 BC 의존 *없어요*.

## 예시

```rust,ignore
use parcel_domain::reader::ParcelReader;
let parcel = reader.fetch_by_pnu(&pnu).await?;
```

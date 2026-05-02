# building-domain

`Building` Aggregate (Core BC, R2 정적 — Reader trait only) crate에요.

## 책임

- 한국 건축물대장 데이터 — V-World/data.go.kr에서 ETL 후 R2에 보관해요.
- Aggregate 자체는 *read-only* — V1 사용자 트래픽 경로에서 변경하지
  않아요. mutation 메서드 0개.
- 한 필지(`Pnu`)에 여러 건물 가능해요 (multi-building per parcel).
- `BuildingPurposeCode` 10종 (산업용 핵심), `BuildingStructureCode` 8종.
- `BuildingReader` trait 포트 — 구현체는 sub-project 4
  (`crates/data-clients/r2-public-data/`)에서 추가해요.
- `ReaderError` enum — `NotFound` / `Fetch` / `Parse`.

## 의존

- `shared-kernel` (`Pnu`, `AreaM2`, `PolygonSrid`).
- 다른 BC 의존 *없어요*.

## 예시

```rust,ignore
use building_domain::reader::BuildingReader;
let buildings = reader.fetch_by_pnu(&pnu).await?;
```

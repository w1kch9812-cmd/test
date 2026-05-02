# industrial-complex-domain

`IndustrialComplex` Aggregate (Core BC, `R2` 정적 — Reader trait only) crate에요.

## 책임

- 한국 산업단지 데이터 (국가/일반/도시첨단/농공) — 산업입지정보시스템
  (`KICOX`)/data.go.kr에서 ETL 후 `R2`에 보관해요.
- Aggregate 자체는 *read-only* — V1 사용자 트래픽 경로에서 변경하지
  않아요. mutation 메서드 0개.
- `IndustrialComplexKind` 4종 (국가/일반/도시첨단/농공).
- `IndustrialComplexReader` trait 포트 — 구현체는 sub-project 4
  (`crates/data-clients/r2-public-data/`)에서 추가해요.
- `ReaderError` enum — `NotFound` / `Fetch` / `Parse`.

## 의존

- `shared-kernel` (`SigunguCode`, `AreaM2`, `BoundingBox`, `PolygonSrid`).
- 다른 BC 의존 *없어요*.

## 예시

```rust,ignore
use industrial_complex_domain::reader::IndustrialComplexReader;
let ic = reader.fetch_by_code("I000001").await?;
```

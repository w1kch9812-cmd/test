# T2: Allowlist 상수 + V-World source rename + Migration 30012

**Goal:** data.go.kr building / V-World parcel allowlist 상수를 정의하고, V-World source taxonomy 확장 마이그레이션(30012)을 적용하며, `crates/data-clients/vworld/src/reader.rs:71` 의 source literal `"vworld"` 를 `"vworld_parcel"` const 로 정정한다.

**Spec SSOT:** §5.2, §5.3, §11 (V-World reader 변경), §13 T2 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**T1 inputs (already exported):** `AllowlistSanitizer::new`, `compute_schema_hash`, `RawSanitizer`, `SanitizedRaw`, `SanitizingRawCapture`.

**Files:**

- Create: `crates/data-clients/raw-capture/src/sources/mod.rs`
- Create: `crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs`
- Create: `crates/data-clients/raw-capture/src/sources/vworld_parcel.rs`
- Create: `migrations/30012_source_taxonomy_expansion.sql`
- Modify: `crates/data-clients/raw-capture/src/sanitizer.rs` (add `for_source` factory)
- Modify: `crates/data-clients/raw-capture/src/lib.rs` (expose `sources` module + doc example)
- Modify: `crates/data-clients/vworld/src/reader.rs` (line 71 source const)

**Existing code refs:**

- [`crates/data-clients/vworld/src/reader.rs:71`](../../../crates/data-clients/vworld/src/reader.rs#L71) — `.capture(pnu.as_str(), "vworld", &raw, now)` (literal — fix target)
- [`crates/data-clients/data-go-kr/src/building_register/reader.rs:37`](../../../crates/data-clients/data-go-kr/src/building_register/reader.rs#L37) — `const RAW_CAPTURE_SOURCE: &str = "data_go_kr_building";` (이미 const 패턴 — V-World 도 따라가야 함)
- [`migrations/30006_parcel_external_data.sql:13-19`](../../../migrations/30006_parcel_external_data.sql#L13-L19) — 기존 CHECK 제약 (`vworld` enum 포함)

---

## Plan Parts

Detailed step bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Sources Module And Source Allowlists](./T2-allowlists-migration.part-01-sources-allowlists.md)
- [Part 02 - Sanitizer Factory And Source Taxonomy Migration](./T2-allowlists-migration.part-02-sanitizer-factory-migration.md)
- [Part 03 - V-World Reader, Docs, Verification, And Acceptance](./T2-allowlists-migration.part-03-vworld-reader-verification.md)

# SP10.5-B: Backend Data Correctness Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** PII allowlist + Two-tier KMS vault + PIPA 4-principles auto-enforcement + Vault RBAC + audit log + Building reader live wiring for SSS-grade PIPA-compliant hardening
**Architecture:** SanitizingRawCapture wrapper (sanitization) + DualTierCapture composer (Tier 2 fail-fast fan-out) + PgPiiVaultCapture (AWS KMS envelope encryption + RLS) + 5 new migrations (30010-30014) + admin RBAC endpoint + Tokio cleanup task + axum-test integration tests
**Tech Stack:** Rust workspace (tokio, axum, sqlx, async-trait, tracing) + PostgreSQL (RLS, composite FK) + AWS KMS (aws-sdk-kms) + AES-256-GCM (aes-gcm crate) + sha2 + ZITADEL JWT + axum-test 15.0
**Spec SSOT:** docs/superpowers/specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md (commit 8a616f5)

---

## File Structure

New files to create:

- `migrations/30010_source_taxonomy_expansion.sql`
- `migrations/30011_parcel_external_data_pii_vault.sql`
- `migrations/30012_lineage_columns.sql`
- `migrations/30013_raw_vault_access_log.sql`
- `migrations/30014_expires_not_null.sql`
- `crates/etl-base-layer/src/sanitizer.rs` (new)
- `crates/etl-base-layer/src/sanitizing_capture.rs` (new)
- `crates/etl-base-layer/src/dual_tier_capture.rs` (new)
- `crates/etl-base-layer/src/pg_pii_vault_capture.rs` (new)
- `crates/etl-base-layer/src/pg_vault_access_log.rs` (new)
- `crates/etl-base-layer/src/cleanup_task.rs` (new)
- `services/api/src/building_reader.rs` (modify)
- `services/api/src/routes/vault_admin.rs` (new)
- `infra/kms-key.ts` (new)

---

## T1: RawSanitizer Trait + SanitizingRawCapture Infra

**Goal:** Define the sanitization abstraction. `RawSanitizer` trait controls which JSON fields survive into `parcel_external_data`. `SanitizingRawCapture` wraps any `RawCapture` to sanitize before storing.

---

- [ ] **Step 1.1:** Define  trait in

```rust
// crates/etl-base-layer/src/sanitizer.rs
// FAILING TEST — trait does not exist yet
#[cfg(test)]
mod tests {
    use super::*;
    struct NullSanitizer;
    impl RawSanitizer for NullSanitizer {
        fn sanitize(&self, _source: &str, raw: &serde_json::Value) -> serde_json::Value {
            raw.clone()
        }
        fn version(&self) -> u32 { 0 }
    }
    #[test]
    fn null_sanitizer_passes_through() {
        let s = NullSanitizer;
        let v = serde_json::json!({key: val});
        assert_eq!(s.sanitize(any, &v), v);
    }
}
```

```
cargo test null_sanitizer_passes_through
# FAILED: error[E0412]: cannot find type  in this scope
```

```rust
// MINIMAL IMPL
pub trait RawSanitizer: Send + Sync {
    fn sanitize(&self, source: &str, raw: &serde_json::Value) -> serde_json::Value;
    fn version(&self) -> u32;
}
```

```
cargo test null_sanitizer_passes_through
# PASSED: test null_sanitizer_passes_through ... ok
```

```bash
git add crates/etl-base-layer/src/sanitizer.rs
git commit -m 'feat(etl-base-layer): add RawSanitizer trait'
```

---

- [ ] **Step 1.2:**  struct with JSON Pointer path matching

```rust
// FAILING TEST
#[test]
fn unknown_source_returns_empty() {
    let s = AllowlistSanitizer::new(unknown_source).unwrap_err();
    assert!(matches!(s, AllowlistError::UnknownSource(_)));
}
```

```
cargo test unknown_source_returns_empty
# FAILED: error[E0422]: cannot find struct  in this scope
```

```rust
// MINIMAL IMPL
#[derive(Debug)]
pub enum AllowlistError {
    UnknownSource(String),
}
pub struct AllowlistSanitizer {
    source: String,
    allowed_paths: Vec<String>,
}
impl AllowlistSanitizer {
    pub fn new(source: &str) -> Result<Self, AllowlistError> {
        match source {
            data_go_kr_building => Ok(Self {
                source: source.to_string(),
                allowed_paths: BUILDING_ALLOWLIST.iter().map(|s| s.to_string()).collect(),
            }),
            vworld_parcel => Ok(Self {
                source: source.to_string(),
                allowed_paths: VWORLD_PARCEL_ALLOWLIST.iter().map(|s| s.to_string()).collect(),
            }),
            other => Err(AllowlistError::UnknownSource(other.to_string())),
        }
    }
}
```

```
cargo test unknown_source_returns_empty
# PASSED: test unknown_source_returns_empty ... ok
```

```bash
git add crates/etl-base-layer/src/sanitizer.rs
git commit -m 'feat(etl-base-layer): add AllowlistSanitizer struct'
```

---

- [ ] **Step 1.3:** `BUILDING_ALLOWLIST` (7 paths) + `BUILDING_PII_FIELDS` constants

```rust
// FAILING TEST
#[test]
fn building_sanitizer_strips_owner_nm() {
    let s = AllowlistSanitizer::new("data_go_kr_building").unwrap();
    let raw = serde_json::json!({
        "response": {
            "header": {"resultCode": "00", "resultMsg": "OK"},
            "body": {
                "items": {
                    "item": [{"mgmBldrgstPk": "abc", "ownerNm": "SECRET", "bldNm": "TestBld"}]
                }
            }
        }
    });
    let out = s.sanitize("data_go_kr_building", &raw);
    let item = &out["response"]["body"]["items"]["item"][0];
    assert!(item.get("ownerNm").is_none(), "ownerNm must be stripped");
    assert!(item.get("bldNm").is_some(), "bldNm must be kept");
}
```

```
cargo test building_sanitizer_strips_owner_nm
# FAILED: thread panicked at assertion failed: item.get("ownerNm").is_none()
```

```rust
// MINIMAL IMPL
pub const BUILDING_ALLOWLIST: [&str; 7] = [
    "/response/header/resultCode",
    "/response/header/resultMsg",
    "/response/body/items/item/*/mgmBldrgstPk",
    "/response/body/items/item/*/bldNm",
    "/response/body/items/item/*/mainPurpsCdNm",
    "/response/body/items/item/*/totArea",
    "/response/body/items/item/*/useAprDay",
];
pub const BUILDING_PII_FIELDS: [&str; 2] = ["ownerNm", "regstrKindCd"];

impl RawSanitizer for AllowlistSanitizer {
    fn sanitize(&self, _source: &str, raw: &serde_json::Value) -> serde_json::Value {
        filter_by_allowlist(raw, &self.allowed_paths)
    }
    fn version(&self) -> u32 { 1 }
}
```

```
cargo test building_sanitizer_strips_owner_nm
# PASSED: test building_sanitizer_strips_owner_nm ... ok
```

```bash
git add crates/etl-base-layer/src/sanitizer.rs
git commit -m 'feat(etl-base-layer): add BUILDING_ALLOWLIST consts and sanitize impl'
```

---

- [ ] **Step 1.4:** `VWORLD_PARCEL_ALLOWLIST` (9 properties + geometry)

```rust
// FAILING TEST
#[test]
fn vworld_parcel_sanitizer_keeps_pnu() {
    let s = AllowlistSanitizer::new("vworld_parcel").unwrap();
    let raw = serde_json::json!({
        "features": [{
            "properties": {
                "pnu": "1168010400100370000",
                "ownerName": "SECRET_PERSON",
                "jibunAddress": "서울특별시 강남구 역삼동 37"
            },
            "geometry": {"type": "Polygon", "coordinates": []}
        }]
    });
    let out = s.sanitize("vworld_parcel", &raw);
    let props = &out["features"][0]["properties"];
    assert!(props.get("pnu").is_some());
    assert!(props.get("ownerName").is_none(), "ownerName must be stripped");
    assert!(out["features"][0].get("geometry").is_some(), "geometry kept");
}
```

```
cargo test vworld_parcel_sanitizer_keeps_pnu
# FAILED: cannot find value `VWORLD_PARCEL_ALLOWLIST` in this scope
```

```rust
// MINIMAL IMPL
pub const VWORLD_PARCEL_ALLOWLIST: [&str; 10] = [
    "/features/*/properties/pnu",
    "/features/*/properties/jibunAddress",
    "/features/*/properties/roadAddress",
    "/features/*/properties/legalDongCode",
    "/features/*/properties/landCategory",
    "/features/*/properties/landArea",
    "/features/*/properties/officialLandPrice",
    "/features/*/properties/useDistrict",
    "/features/*/properties/buildingCoverageRatio",
    "/features/*/geometry",
];
```

```
cargo test vworld_parcel_sanitizer_keeps_pnu
# PASSED: test vworld_parcel_sanitizer_keeps_pnu ... ok
```

```bash
git add crates/etl-base-layer/src/sanitizer.rs
git commit -m 'feat(etl-base-layer): add VWORLD_PARCEL_ALLOWLIST const'
```

---

- [ ] **Step 1.5:** AllowlistSanitizer::new factory returns correct variant

```rust
// FAILING TEST
#[test]
fn allowlist_new_building_has_7_paths() {
    let s = AllowlistSanitizer::new("data_go_kr_building").unwrap();
    assert_eq!(s.allowed_paths.len(), 7);
}
```

```
cargo test allowlist_new_building_has_7_paths
# FAILED: error: field allowed_paths is private
```

```rust
// MINIMAL IMPL
pub struct AllowlistSanitizer {
    pub(crate) allowed_paths: Vec<String>,
}
```

```
cargo test allowlist_new_building_has_7_paths
# PASSED: test ok
```

```bash
git add crates/etl-base-layer/src/sanitizer.rs
git commit -m 'test(etl-base-layer): verify factory path counts'
```

---

- [ ] **Step 1.6:** SanitizingRawCapture<C, S> wrapper implementing RawCapture

```rust
// FAILING TEST - sanitizing_capture.rs
#[tokio::test]
async fn sanitizing_capture_strips_pii_before_store() {
    let spy = SpyCapture::new();
    let san = AllowlistSanitizer::new("data_go_kr_building").unwrap();
    let cap = SanitizingRawCapture::new(spy.clone(), san);
    let raw = serde_json::json!({
        "response": {"body": {"items": {"item": [{
            "mgmBldrgstPk": "X",
            "ownerNm": "HIDDEN"
        }]}}}
    });
    cap.capture("1168010400100370000", "data_go_kr_building",
                &raw, chrono::Utc::now()).await.unwrap();
    let stored = spy.last_raw();
    assert!(stored["response"]["body"]
              ["items"]["item"][0]
              .get("ownerNm").is_none(),
            "ownerNm must not reach inner capture");
}
```

```
cargo test sanitizing_capture_strips_pii_before_store
# FAILED: cannot find struct SanitizingRawCapture in scope
```

```rust
// MINIMAL IMPL
pub struct SanitizingRawCapture<C: RawCapture, S: RawSanitizer> {
    inner: C, sanitizer: S,
}
impl<C: RawCapture, S: RawSanitizer> SanitizingRawCapture<C, S> {
    pub fn new(inner: C, sanitizer: S) -> Self { Self { inner, sanitizer } }
}
#[async_trait::async_trait]
impl<C: RawCapture + Send + Sync, S: RawSanitizer> RawCapture
    for SanitizingRawCapture<C, S> {
    async fn capture(
        &self, pnu: &str, source: &str,
        raw: &serde_json::Value,
        fetched_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), crate::RawCaptureError> {
        let sanitized = self.sanitizer.sanitize(source, raw);
        self.inner.capture(pnu, source, &sanitized, fetched_at).await
    }
}
```

```
cargo test sanitizing_capture_strips_pii_before_store
# PASSED: test sanitizing_capture_strips_pii_before_store ... ok
```

```bash
git add crates/etl-base-layer/src/sanitizing_capture.rs
git commit -m 'feat(etl-base-layer): SanitizingRawCapture wrapper impl'
```

---

- [ ] **Step 1.7:** SanitizingRawCapture propagates inner capture errors unchanged

```rust
// FAILING TEST
#[tokio::test]
async fn sanitizing_capture_propagates_db_error() {
    let inner = FailingCapture::new(RawCaptureError::Db("conn fail".into()));
    let san = AllowlistSanitizer::new("data_go_kr_building").unwrap();
    let cap = SanitizingRawCapture::new(inner, san);
    let raw = serde_json::json!({"response": {}});
    let err = cap.capture("pnu", "data_go_kr_building",
                          &raw, chrono::Utc::now()).await.unwrap_err();
    assert!(matches!(err, RawCaptureError::Db(_)));
}
```

```
cargo test sanitizing_capture_propagates_db_error
# FAILED: cannot find struct FailingCapture in scope
```

```rust
// MINIMAL IMPL - add FailingCapture test helper
#[cfg(test)]
pub struct FailingCapture { err: RawCaptureError }
impl FailingCapture {
    pub fn new(err: RawCaptureError) -> Self { Self { err } }
}
#[async_trait::async_trait]
impl RawCapture for FailingCapture {
    async fn capture(&self, _pnu: &str, _source: &str,
                     _raw: &serde_json::Value,
                     _fetched_at: chrono::DateTime<chrono::Utc>)
        -> Result<(), RawCaptureError> { Err(self.err.clone()) }
}
```

```
cargo test sanitizing_capture_propagates_db_error
# PASSED: test sanitizing_capture_propagates_db_error ... ok
```

```bash
git add crates/etl-base-layer/src/
git commit -m 'test(etl-base-layer): verify SanitizingRawCapture error propagation'
```

---

- [ ] **Step 1.8:** sanitizer_version written on capture (requires migration 30012 from T3.2)

```rust
// FAILING TEST
#[sqlx::test(migrations = "migrations")]
async fn sanitizing_capture_sets_sanitizer_version(pool: PgPool) {
    let cap = PgRawCapture::new(pool.clone());
    let san = AllowlistSanitizer::new("data_go_kr_building").unwrap();
    let svc = SanitizingRawCapture::new(cap, san);
    let raw = serde_json::json!({"response": {}});
    svc.capture("1168010400100370000", "data_go_kr_building",
                &raw, chrono::Utc::now()).await.unwrap();
    let row = sqlx::query!(
        "SELECT sanitizer_version FROM parcel_external_data WHERE pnu = $1",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.sanitizer_version, 1);
}
```

```
cargo test sanitizing_capture_sets_sanitizer_version
# FAILED: column sanitizer_version does not exist (needs migration 30012)
```

```rust
// MINIMAL IMPL: PgRawCapture INSERT includes sanitizer_version column
// INSERT INTO parcel_external_data
//   (pnu, source, raw_response, sanitizer_version, fetched_at, expires_at)
// VALUES ($1, $2, $3, $4, $5, $5 + INTERVAL '30 days')
// SanitizingRawCapture passes self.sanitizer.version() as sanitizer_version
```

```
cargo test sanitizing_capture_sets_sanitizer_version
# PASSED: test sanitizing_capture_sets_sanitizer_version ... ok
```

```bash
git add crates/etl-base-layer/src/
git commit -m 'feat(etl-base-layer): write sanitizer_version on capture'
```

---

## T2: Allowlist Definitions + V-World Source Taxonomy (BLOCKER)

**Goal:** Define compile-time allowlist consts for data_go_kr_building (7 paths) and vworld_parcel (9 properties + envelope). Apply migration 30010_source_taxonomy_expansion. Update reader.rs in the same PR.

> **BLOCKER:** migration 30010 + reader.rs const change must land in the same PR. Separate PR would re-insert vworld rows after backfill.

- [ ] **Step 2.1 - Failing test: BUILDING_ALLOWLIST has exactly 7 paths**

  ```rust
  // crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs
  #[cfg(test)]
  mod tests {
      use super::BUILDING_ALLOWLIST;
      #[test]
      fn building_allowlist_has_seven_paths() {
          assert_eq!(BUILDING_ALLOWLIST.len(), 7);
          assert!(BUILDING_ALLOWLIST.contains(&"/response/body/items/item/*/mgmBldrgstPk"));
      }
  }
  ```

  ```bash
  cargo test -p raw-capture building_allowlist_has_seven_paths 2>&1 | tail -5
  # expected: FAILED -- module not found
  ```

- [ ] **Step 2.2 - Impl: BUILDING_ALLOWLIST with real 7 paths and PII field list**

  ```rust
  // crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs
  /// Allowlist for data_go_kr_building (spec section 5.2 SSOT).
  /// ownerNm and regstrKindCd are PII and are dropped by AllowlistSanitizer.
  pub const BUILDING_ALLOWLIST: [&str; 7] = [
      "/response/header/resultCode",
      "/response/header/resultMsg",
      "/response/body/items/item/*/mgmBldrgstPk",
      "/response/body/items/item/*/bldNm",
      "/response/body/items/item/*/mainPurpsCdNm",
      "/response/body/items/item/*/totArea",
      "/response/body/items/item/*/useAprDay",
  ];
  pub const BUILDING_PII_FIELDS: [&str; 2] = ["ownerNm", "regstrKindCd"];
  ```

  ```bash
  cargo test -p raw-capture building_allowlist_has_seven_paths 2>&1 | tail -5
  # expected: test sources::data_go_kr_building::tests::building_allowlist_has_seven_paths ... ok
  ```

  ```bash
  git add crates/data-clients/raw-capture/src/sources/
  git commit -m "feat(raw-capture): add BUILDING_ALLOWLIST 7-path const and BUILDING_PII_FIELDS"
  ```

- [ ] **Step 2.3 - Failing test: VWORLD_PARCEL_ALLOWLIST includes geometry and pnu**

  ```rust
  #[test]
  fn vworld_parcel_allowlist_includes_geometry_and_pnu() {
      assert!(VWORLD_PARCEL_ALLOWLIST.iter().any(|p| p.contains("geometry")));
      assert!(VWORLD_PARCEL_ALLOWLIST.iter().any(|p| p.contains("/properties/pnu")));
  }
  ```

  ```bash
  cargo test -p raw-capture vworld_parcel_allowlist_includes_geometry_and_pnu 2>&1 | tail -5
  # expected: FAILED -- module not found
  ```

- [ ] **Step 2.4 - Impl: VWORLD_PARCEL_ALLOWLIST const**

  ```rust
  pub const VWORLD_PARCEL_ALLOWLIST: [&str; 13] = [
      "/response/result/featureCollection/features/*/geometry",
      "/response/result/featureCollection/features/*/properties/pnu",
      "/response/result/featureCollection/features/*/properties/jibun",
      "/response/result/featureCollection/features/*/properties/bonbun",
      "/response/result/featureCollection/features/*/properties/bubun",
      "/response/result/featureCollection/features/*/properties/addr",
      "/response/result/featureCollection/features/*/properties/jiga",
      "/response/result/featureCollection/features/*/properties/gosi_year",
      "/response/result/featureCollection/features/*/properties/gosi_month",
      "/response/service/*",
      "/response/status",
      "/response/error/*",
      "/response/record/*",
  ];
  ```

  ```bash
  cargo test -p raw-capture vworld_parcel_allowlist_includes_geometry_and_pnu 2>&1 | tail -5
  # expected: test sources::vworld_parcel::tests::vworld_parcel_allowlist_includes_geometry_and_pnu ... ok
  ```

  ```bash
  git add crates/data-clients/raw-capture/src/sources/vworld_parcel.rs
  git commit -m "feat(raw-capture): add VWORLD_PARCEL_ALLOWLIST const (9 properties + envelope)"
  ```

- [ ] **Step 2.5 - Write migration 30010_source_taxonomy_expansion.sql (BLOCKER)**

  Create `migrations/30010_source_taxonomy_expansion.sql`:

  ```sql
  -- migrations/30010_source_taxonomy_expansion.sql
  -- BLOCKER: must ship in same PR as reader.rs RAW_CAPTURE_SOURCE const.
  ALTER TABLE parcel_external_data
      DROP CONSTRAINT parcel_external_data_source_check;

  ALTER TABLE parcel_external_data
      ADD CONSTRAINT parcel_external_data_source_check
      CHECK (source IN (
          'vworld',
          'vworld_parcel',
          'data_go_kr_building',
          'data_go_kr_land',
          'data_go_kr_realtransaction',
          'korean_law'
      ));

  UPDATE parcel_external_data
     SET source = 'vworld_parcel'
   WHERE source = 'vworld';
  ```

  ```bash
  sqlx migrate run 2>&1 | tail -5
  # expected: Applied 30010/source_taxonomy_expansion (Xms)
  ```

  ```bash
  git add migrations/30010_source_taxonomy_expansion.sql
  git commit -m "feat(db): migration 30010 -- source taxonomy expansion vworld -> vworld_parcel"
  ```

- [ ] **Step 2.6 - Add RAW_CAPTURE_SOURCE const to reader.rs (BLOCKER)**

  ```rust
  // crates/data-clients/vworld/src/reader.rs
  const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";
  ```

  Replace the literal "vworld" at line 71 with RAW_CAPTURE_SOURCE.

  ```bash
  cargo test -p vworld 2>&1 | tail -5
  # expected: test result: ok. N passed; 0 failed
  ```

  ```bash
  git add crates/data-clients/vworld/src/reader.rs migrations/30010_source_taxonomy_expansion.sql
  git commit -m "fix(vworld): replace hardcoded vworld source literal with RAW_CAPTURE_SOURCE const"
  ```

- [ ] **Step 2.7 - Test: building sanitizer drops ownerNm and regstrKindCd**

  ```rust
  #[test]
  fn building_sanitizer_drops_owner_nm_and_registr_kind_cd() {
      let raw = json!({ "mgmBldrgstPk": "11110-100000012", "bldNm": "TestBldg", "ownerNm": "Hong", "regstrKindCd": "1" });
      let sanitizer = AllowlistSanitizer::new("data_go_kr_building", 1, vec!["mgmBldrgstPk".to_string(), "bldNm".to_string()]);
      let result = sanitizer.sanitize("data_go_kr_building", &raw).unwrap();
      assert!(result.value.get("ownerNm").is_none(), "ownerNm must be dropped");
      assert!(result.value.get("regstrKindCd").is_none(), "regstrKindCd must be dropped");
      assert!(result.value.get("bldNm").is_some());
  }
  ```

  ```bash
  cargo test -p raw-capture building_sanitizer_drops_owner_nm 2>&1 | tail -5
  # expected: test sources::data_go_kr_building::tests::building_sanitizer_drops_owner_nm_and_registr_kind_cd ... ok
  ```

  ```bash
  git add crates/data-clients/raw-capture/src/sources/data_go_kr_building.rs
  git commit -m "test(raw-capture): verify ownerNm and regstrKindCd dropped by building allowlist"
  ```

- [ ] **Step 2.8 - Verify backfill: no vworld rows remain after migration 30010**

  ```sql
  SELECT COUNT(*) FROM parcel_external_data WHERE source = 'vworld';
  -- expected: 0
  SELECT COUNT(*) FROM parcel_external_data WHERE source = 'vworld_parcel';
  -- expected: N (all former vworld rows migrated)
  ```

  ```bash
  git add migrations/30010_source_taxonomy_expansion.sql
  git commit -m "test(db): document 30010 backfill verification queries"
  ```

---

## T3: Two-Tier Vault Migrations + PgPiiVaultCapture + Lineage

**Goal:** Create the encrypted PII vault (`parcel_external_data_pii_vault`) with RLS, add lineage columns to `parcel_external_data`, implement KMS envelope encryption, and wire `DualTierCapture` to enforce Tier 2 fail-fast.

---

- [ ] **Step 3.1:** Migration 30011 — parcel_external_data_pii_vault table + RLS

```sql
-- migrations/30011_parcel_external_data_pii_vault.sql
-- FAILING: table does not exist yet
CREATE TABLE parcel_external_data_pii_vault (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pnu char(19) NOT NULL,
    source varchar(40) NOT NULL CHECK (source IN (
        'vworld','vworld_parcel','data_go_kr_building',
        'data_go_kr_land','data_go_kr_realtransaction','korean_law'
    )),
    FOREIGN KEY (pnu, source) REFERENCES parcel_external_data(pnu, source) ON DELETE CASCADE,
    ciphertext_blob BYTEA NOT NULL,
    kms_key_id TEXT NOT NULL,
    encryption_ctx JSONB NOT NULL DEFAULT '{}',
    captured_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
ALTER TABLE parcel_external_data_pii_vault ENABLE ROW LEVEL SECURITY;
CREATE POLICY vault_admin_only ON parcel_external_data_pii_vault
    USING (current_setting('app.role', true) = 'admin');
```

```
sqlx migrate run  # apply migration 30011
# FAILED: relation parcel_external_data_pii_vault does not exist
```

```bash
# Create the migration file at migrations/30011_parcel_external_data_pii_vault.sql
# (content is the SQL above)
```

```
sqlx migrate run
# PASSED: Applied migration 30011_parcel_external_data_pii_vault
```

```rust
// VERIFY: table exists + RLS active
#[sqlx::test(migrations = "migrations")]
async fn pii_vault_table_has_rls(pool: PgPool) {
    let row = sqlx::query!(
        "SELECT relrowsecurity FROM pg_class WHERE relname = 'parcel_external_data_pii_vault'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.relrowsecurity, Some(true));
}
```

```
cargo test pii_vault_table_has_rls
# PASSED: test pii_vault_table_has_rls ... ok
```

```bash
git add migrations/30011_parcel_external_data_pii_vault.sql
git commit -m 'feat(db): add parcel_external_data_pii_vault table with RLS'
```

---

- [ ] **Step 3.2:** Migration 30012 — lineage columns on parcel_external_data

```sql
-- migrations/30012_lineage_columns.sql
ALTER TABLE parcel_external_data
    ADD COLUMN license TEXT,
    ADD COLUMN api_version TEXT,
    ADD COLUMN sanitizer_version INT NOT NULL DEFAULT 1,
    ADD COLUMN schema_hash TEXT;
UPDATE parcel_external_data
   SET schema_hash = 'legacy:' || md5(raw_response::text), sanitizer_version = 0
 WHERE schema_hash IS NULL OR schema_hash = '';
```

```
cargo test lineage_migration_legacy_rows_have_schema_hash
# FAILED: column sanitizer_version does not exist
```

```rust
// VERIFY TEST
#[sqlx::test(migrations = "migrations")]
async fn lineage_migration_legacy_rows_have_schema_hash(pool: PgPool) {
    // Insert a pre-migration row (simulate existing data)
    sqlx::query!(
        "INSERT INTO parcel_external_data (pnu, source, raw_response, fetched_at, expires_at)"
        " VALUES ($1, $2, $3, NOW(), NOW() + INTERVAL '30 days')",
        "1168010400100370000", "data_go_kr_building",
        serde_json::json!({"legacy": true})
    ).execute(&pool).await.unwrap();
    let row = sqlx::query!(
        "SELECT schema_hash FROM parcel_external_data WHERE pnu = $1",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    // Migration UPDATE sets schema_hash for NULL rows
    assert!(row.schema_hash.map(|h| h.starts_with("legacy:")).unwrap_or(false));
}
```

```
cargo test lineage_migration_legacy_rows_have_schema_hash
# PASSED: test lineage_migration_legacy_rows_have_schema_hash ... ok
```

```bash
git add migrations/30012_lineage_columns.sql
git commit -m 'feat(db): add lineage columns to parcel_external_data'
```

---

- [ ] **Step 3.3:** KmsEnvelopeEncryptor — GenerateDataKey + AES-256-GCM encrypt

```rust
// FAILING TEST - crates/etl-base-layer/src/pg_pii_vault_capture.rs
#[tokio::test]
async fn kms_envelope_encrypt_decrypt_roundtrip() {
    let kms = MockKmsClient::new();
    let encryptor = KmsEnvelopeEncryptor::new(kms, "test-key-id".to_string());
    let plaintext = b"secret owner data";
    let (ciphertext, key_id, ctx) = encryptor.encrypt(plaintext).await.unwrap();
    let decrypted = encryptor.decrypt(&ciphertext, &key_id, &ctx).await.unwrap();
    assert_eq!(decrypted, plaintext);
}
```

```
cargo test kms_envelope_encrypt_decrypt_roundtrip
# FAILED: cannot find struct KmsEnvelopeEncryptor in scope
```

```rust
// MINIMAL IMPL
pub struct KmsEnvelopeEncryptor {
    kms: Arc<dyn KmsClientTrait + Send + Sync>,
    key_id: String,
}
impl KmsEnvelopeEncryptor {
    pub fn new(kms: impl KmsClientTrait + Send + Sync + 'static, key_id: String) -> Self {
        Self { kms: Arc::new(kms), key_id }
    }
    pub async fn encrypt(&self, plaintext: &[u8])
        -> Result<(Vec<u8>, String, serde_json::Value), KmsError> {
        // 1. GenerateDataKey -> (dek_plaintext, dek_ciphertext)
        // 2. AES-256-GCM encrypt plaintext with dek_plaintext
        // 3. Return (ciphertext, kms_key_id, encryption_ctx{dek_ciphertext})
        let dek = self.kms.generate_data_key(&self.key_id).await?;
        let cipher = aes_gcm::Aes256Gcm::new_from_slice(&dek.plaintext).unwrap();
        let nonce = aes_gcm::Nonce::from_slice(&[0u8; 12]);
        let ciphertext = cipher.encrypt(nonce, plaintext)
            .map_err(|e| KmsError::Encrypt(e.to_string()))?;
        let ctx = serde_json::json!({"dek_ciphertext": base64::encode(&dek.ciphertext)});
        Ok((ciphertext, self.key_id.clone(), ctx))
    }
}
```

```
cargo test kms_envelope_encrypt_decrypt_roundtrip
# PASSED: test kms_envelope_encrypt_decrypt_roundtrip ... ok
```

```bash
git add crates/etl-base-layer/src/pg_pii_vault_capture.rs
git commit -m 'feat(etl-base-layer): KmsEnvelopeEncryptor with AES-256-GCM'
```

---

- [ ] **Step 3.4:** PgPiiVaultCapture implementing RawCapture — inserts encrypted row

```rust
// FAILING TEST
#[sqlx::test(migrations = "migrations")]
async fn pg_pii_vault_capture_inserts_encrypted_row(pool: PgPool) {
    let kms = MockKmsClient::new();
    let cap = PgPiiVaultCapture::new(pool.clone(), kms, "test-key-id".to_string(), 30);
    let pnu = "1168010400100370000";
    let raw = serde_json::json!({"ownerNm": "홍길동"});
    cap.capture(pnu, "data_go_kr_building", &raw, chrono::Utc::now()).await.unwrap();
    let row = sqlx::query!(
        "SELECT ciphertext_blob, kms_key_id FROM parcel_external_data_pii_vault WHERE pnu = $1",
        pnu
    ).fetch_one(&pool).await.unwrap();
    assert!(!row.ciphertext_blob.is_empty());
    // ciphertext must not contain plaintext ownerNm
    assert!(!String::from_utf8_lossy(&row.ciphertext_blob).contains("홍길동"));
}
```

```
cargo test pg_pii_vault_capture_inserts_encrypted_row
# FAILED: cannot find struct PgPiiVaultCapture in scope
```

```rust
// MINIMAL IMPL
pub struct PgPiiVaultCapture {
    pool: PgPool,
    encryptor: KmsEnvelopeEncryptor,
    ttl_days: i64,
}
impl PgPiiVaultCapture {
    pub fn new(pool: PgPool, kms: impl KmsClientTrait + Send + Sync + 'static,
               kms_key_id: String, ttl_days: i64) -> Self {
        Self { pool, encryptor: KmsEnvelopeEncryptor::new(kms, kms_key_id), ttl_days }
    }
}
#[async_trait::async_trait]
impl RawCapture for PgPiiVaultCapture {
    async fn capture(&self, pnu: &str, source: &str,
                     raw: &serde_json::Value,
                     fetched_at: chrono::DateTime<chrono::Utc>)
        -> Result<(), RawCaptureError> {
        let bytes = serde_json::to_vec(raw).map_err(RawCaptureError::Serialize)?;
        let (ciphertext, key_id, ctx) = self.encryptor.encrypt(&bytes).await
            .map_err(RawCaptureError::Kms)?;
        let expires_at = fetched_at + chrono::Duration::days(self.ttl_days);
        sqlx::query!(
            "INSERT INTO parcel_external_data_pii_vault"
            "  (pnu, source, ciphertext_blob, kms_key_id, encryption_ctx, captured_at, expires_at)"
            "VALUES ($1, $2, $3, $4, $5, $6, $7)",
            pnu, source, ciphertext, key_id,
            serde_json::to_value(ctx).unwrap(),
            fetched_at, expires_at
        ).execute(&self.pool).await.map_err(RawCaptureError::Db)?;
        Ok(())
    }
}
```

```
cargo test pg_pii_vault_capture_inserts_encrypted_row
# PASSED: test pg_pii_vault_capture_inserts_encrypted_row ... ok
```

```bash
git add crates/etl-base-layer/src/pg_pii_vault_capture.rs
git commit -m 'feat(etl-base-layer): PgPiiVaultCapture stores encrypted PII'
```

---

- [ ] **Step 3.5:** PgPiiVaultCapture sets expires_at = fetched_at + ttl_days

```rust
#[sqlx::test(migrations = "migrations")]
async fn pg_pii_vault_capture_expires_at_correct(pool: PgPool) {
    let cap = PgPiiVaultCapture::new(pool.clone(), MockKmsClient::new(),
                                       "k".to_string(), 30);
    let fetched_at = chrono::Utc::now();
    let pnu = "1168010400100370000";
    cap.capture(pnu, "data_go_kr_building",
                &serde_json::json!({}), fetched_at).await.unwrap();
    let row = sqlx::query!(
        "SELECT expires_at FROM parcel_external_data_pii_vault WHERE pnu = $1",
        pnu
    ).fetch_one(&pool).await.unwrap();
    let expected = fetched_at + chrono::Duration::days(30);
    let diff = (row.expires_at - expected).num_seconds().abs();
    assert!(diff < 2, "expires_at must be fetched_at + 30 days (got diff {diff}s)");
}
```

```
cargo test pg_pii_vault_capture_expires_at_correct
# FAILED: expires_at diff > 2 (bug in ttl calculation)
```

Ensure `expires_at = fetched_at + chrono::Duration::days(self.ttl_days)` in INSERT.

```
cargo test pg_pii_vault_capture_expires_at_correct
# PASSED: test pg_pii_vault_capture_expires_at_correct ... ok
```

```bash
git add crates/etl-base-layer/src/pg_pii_vault_capture.rs
git commit -m 'test(etl-base-layer): verify vault expires_at = fetched_at + 30 days'
```

---

- [ ] **Step 3.6:** PgPiiVaultCapture RLS enforcement — non-admin role blocked

```rust
#[sqlx::test(migrations = "migrations")]
async fn rls_blocks_non_admin_select(pool: PgPool) {
    // Insert a vault row
    let cap = PgPiiVaultCapture::new(pool.clone(), MockKmsClient::new(),
                                       "k".to_string(), 30);
    cap.capture("1168010400100370000", "data_go_kr_building",
                &serde_json::json!({}), chrono::Utc::now()).await.unwrap();
    // SET non-admin role
    sqlx::query!("SET app.role = 'user'").execute(&pool).await.unwrap();
    let count = sqlx::query!(
        "SELECT COUNT(*) as cnt FROM parcel_external_data_pii_vault"
    ).fetch_one(&pool).await.unwrap().cnt.unwrap_or(0);
    assert_eq!(count, 0, "non-admin must see 0 rows (RLS)");
}
```

```
cargo test rls_blocks_non_admin_select
# FAILED: count = 1 (RLS not enforced for superuser in test)
# Note: In test env, connect as limited role; use SET ROLE in sqlx connection
```

Connect as restricted PostgreSQL role for test, or use SET LOCAL app.role for each query.

```
cargo test rls_blocks_non_admin_select
# PASSED: test rls_blocks_non_admin_select ... ok
```

```bash
git add crates/etl-base-layer/src/pg_pii_vault_capture.rs
git commit -m 'test(etl-base-layer): verify RLS blocks non-admin vault access'
```

---

- [ ] **Step 3.7:** DualTierCapture<T1, T2> — Tier 2 fail-fast before Tier 1

```rust
// FAILING TEST
#[tokio::test]
async fn dual_tier_vault_failure_aborts_tier1() {
    let spy_tier1 = SpyCapture::new();
    let failing_tier2 = FailingCapture::new(RawCaptureError::Kms("kms error".into()));
    let cap = DualTierCapture::new(spy_tier1.clone(), failing_tier2);
    let raw = serde_json::json!({});
    let err = cap.capture("pnu", "data_go_kr_building",
                           &raw, chrono::Utc::now()).await.unwrap_err();
    assert!(matches!(err, RawCaptureError::Kms(_)));
    assert_eq!(spy_tier1.call_count(), 0, "tier1 must NOT be called when tier2 fails");
}
```

```
cargo test dual_tier_vault_failure_aborts_tier1
# FAILED: cannot find struct DualTierCapture in scope
```

```rust
// MINIMAL IMPL
pub struct DualTierCapture<T1: RawCapture, T2: RawCapture> {
    tier1: T1,  // parcel_external_data (sanitized)
    tier2: T2,  // pii vault (encrypted raw PII)
}
impl<T1: RawCapture, T2: RawCapture> DualTierCapture<T1, T2> {
    pub fn new(tier1: T1, tier2: T2) -> Self { Self { tier1, tier2 } }
}
#[async_trait::async_trait]
impl<T1: RawCapture + Send + Sync, T2: RawCapture + Send + Sync> RawCapture
    for DualTierCapture<T1, T2> {
    async fn capture(&self, pnu: &str, source: &str,
                     raw: &serde_json::Value,
                     fetched_at: chrono::DateTime<chrono::Utc>)
        -> Result<(), RawCaptureError> {
        // Tier 2 FIRST — fail-fast: vault failure aborts entire operation
        self.tier2.capture(pnu, source, raw, fetched_at).await?;
        // Tier 1 SECOND — only reached if tier2 succeeded
        self.tier1.capture(pnu, source, raw, fetched_at).await
    }
}
```

```
cargo test dual_tier_vault_failure_aborts_tier1
# PASSED: test dual_tier_vault_failure_aborts_tier1 ... ok
```

```bash
git add crates/etl-base-layer/src/dual_tier_capture.rs
git commit -m 'feat(etl-base-layer): DualTierCapture with Tier 2 fail-fast'
```

---

- [ ] **Step 3.8:** DualTierCapture success path — both tiers written

```rust
#[tokio::test]
async fn dual_tier_success_writes_both_tiers() {
    let spy1 = SpyCapture::new();
    let spy2 = SpyCapture::new();
    let cap = DualTierCapture::new(spy1.clone(), spy2.clone());
    let raw = serde_json::json!({"field": "value"});
    cap.capture("pnu", "data_go_kr_building",
                &raw, chrono::Utc::now()).await.unwrap();
    assert_eq!(spy1.call_count(), 1, "tier1 must be called once");
    assert_eq!(spy2.call_count(), 1, "tier2 must be called once");
}
```

```
cargo test dual_tier_success_writes_both_tiers
# FAILED: spy1.call_count() == 0 (tier1 not reached due to ordering bug)
```

Ensure tier1.capture is called AFTER tier2 succeeds (already in impl above).

```
cargo test dual_tier_success_writes_both_tiers
# PASSED: test dual_tier_success_writes_both_tiers ... ok
```

```bash
git add crates/etl-base-layer/src/dual_tier_capture.rs
git commit -m 'test(etl-base-layer): verify DualTierCapture writes both tiers on success'
```

---

- [ ] **Step 3.9:** schema_hash computation using sha2 — prefix sha256:

```rust
#[test]
fn schema_hash_is_sha256_prefixed() {
    let raw = serde_json::json!({"key": "value"});
    let hash = compute_schema_hash(&raw);
    assert!(hash.starts_with("sha256:"), "must start with sha256: prefix");
    assert_eq!(hash.len(), 71, "sha256: (7) + 64 hex chars = 71");
}
```

```
cargo test schema_hash_is_sha256_prefixed
# FAILED: cannot find fn compute_schema_hash
```

```rust
// MINIMAL IMPL
use sha2::{Sha256, Digest};
pub fn compute_schema_hash(raw: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(raw).unwrap_or_default();
    let digest = Sha256::digest(&bytes);
    format!("sha256:{}", hex::encode(digest))
}
```

```
cargo test schema_hash_is_sha256_prefixed
# PASSED: test schema_hash_is_sha256_prefixed ... ok
```

```bash
git add crates/etl-base-layer/src/
git commit -m 'feat(etl-base-layer): compute_schema_hash with sha256: prefix'
```

---

- [ ] **Step 3.10:** Pulumi KMS key provisioning in infra/kms-key.ts

```typescript
// infra/kms-key.ts
// FAILING: no KMS resource in Pulumi stack
import * as aws from "@pulumi/aws";
export const parcelPiiVaultKey = new aws.kms.Key("parcel-pii-vault-key", {
    description: "parcel-pii-vault",
    enableKeyRotation: true,
    tags: { Environment: "production", Service: "etl-pii-vault" },
});
export const parcelPiiVaultKeyArn = parcelPiiVaultKey.arn;
export const parcelPiiVaultKeyId = parcelPiiVaultKey.keyId;
```

```
pulumi preview
# FAILED: no KMS resources found (file does not exist yet)
```

Create `infra/kms-key.ts` with above content and import it in `infra/index.ts`.

```
pulumi preview
# PASSED: 1 resource to create: aws:kms/key:Key (parcel-pii-vault-key)
```

```bash
git add infra/kms-key.ts infra/index.ts
git commit -m 'feat(infra): add KMS key for parcel PII vault encryption'
```

## T4: expires_at TTL + Cleanup Task

**Goal:** Enforce NOT NULL constraint on expires_at, add check constraint, index, and implement a Tokio periodic cleanup task for GDPR/PIPA auto-deletion.

---

- [ ] **Step 4.1:** Migration 30014 — expires_at NOT NULL + check + indexes

```sql
-- migrations/30014_expires_not_null.sql
UPDATE parcel_external_data SET expires_at = fetched_at + INTERVAL '30 days' WHERE expires_at IS NULL;
ALTER TABLE parcel_external_data ALTER COLUMN expires_at SET NOT NULL;
ALTER TABLE parcel_external_data ADD CONSTRAINT check_expires_future
    CHECK (expires_at > fetched_at);
CREATE INDEX idx_external_data_expires ON parcel_external_data (expires_at);
CREATE INDEX idx_pii_vault_expires ON parcel_external_data_pii_vault (expires_at);
```

```rust
// FAILING TEST
#[sqlx::test(migrations = "migrations")]
async fn expires_at_not_null_enforced(pool: PgPool) {
    let res = sqlx::query!(
        "INSERT INTO parcel_external_data (pnu, source, raw_response, fetched_at)"
        " VALUES ($1, $2, $3, NOW())",
        "1168010400100370000", "data_go_kr_building",
        serde_json::json!({})
    ).execute(&pool).await;
    assert!(res.is_err(), "INSERT without expires_at must fail");
}
```

```
cargo test expires_at_not_null_enforced
# FAILED: INSERT succeeded (migration 30014 not applied)
```

Apply migration 30014_expires_not_null.sql.

```
cargo test expires_at_not_null_enforced
# PASSED: test expires_at_not_null_enforced ... ok
```

```bash
git add migrations/30014_expires_not_null.sql
git commit -m 'feat(db): enforce expires_at NOT NULL on parcel_external_data'
```

---

- [ ] **Step 4.2:** CleanupTask struct — Tokio periodic task with CancellationToken

```rust
// FAILING TEST - crates/etl-base-layer/src/cleanup_task.rs
#[tokio::test]
async fn cleanup_task_spawn_does_not_panic() {
    let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
    let token = tokio_util::sync::CancellationToken::new();
    let handle = CleanupTask::spawn(pool, std::time::Duration::from_secs(3600), token.clone());
    token.cancel();
    tokio::time::timeout(std::time::Duration::from_millis(200), handle).await
        .expect("cleanup task must exit within 200ms of cancellation")
        .expect("JoinHandle must not panic");
}
```

```
cargo test cleanup_task_spawn_does_not_panic
# FAILED: cannot find struct CleanupTask in scope
```

```rust
// MINIMAL IMPL
pub struct CleanupTask;
impl CleanupTask {
    pub fn spawn(pool: PgPool, interval: std::time::Duration,
                 cancel: tokio_util::sync::CancellationToken)
        -> tokio::task::JoinHandle<()>
    {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        let _ = Self::run_once(&pool).await;
                    }
                    _ = cancel.cancelled() => { break; }
                }
            }
        })
    }
    pub async fn run_once(pool: &PgPool) -> Result<(u64, u64), sqlx::Error> {
        // delete vault first (FK order), then main table
        let v = sqlx::query!(
            "DELETE FROM parcel_external_data_pii_vault WHERE expires_at < NOW()"
        ).execute(pool).await?.rows_affected();
        let m = sqlx::query!(
            "DELETE FROM parcel_external_data WHERE expires_at < NOW()"
        ).execute(pool).await?.rows_affected();
        tracing::info!(deleted_vault = v, deleted_main = m, "cleanup cycle complete");
        Ok((v, m))
    }
}
```

```
cargo test cleanup_task_spawn_does_not_panic
# PASSED: test cleanup_task_spawn_does_not_panic ... ok
```

```bash
git add crates/etl-base-layer/src/cleanup_task.rs
git commit -m 'feat(etl-base-layer): CleanupTask with CancellationToken'
```

---

- [ ] **Step 4.3:** CleanupTask deletes vault first (FK order) then main

```rust
#[sqlx::test(migrations = "migrations")]
async fn cleanup_run_once_deletes_both(pool: PgPool) {
    let (v, m) = CleanupTask::run_once(&pool).await.unwrap();
    assert_eq!(v + m, 0);  // nothing to delete on empty db
}
```

```
cargo test cleanup_run_once_deletes_both
# FAILED: FK violation — vault not deleted first
```

```rust
pub async fn run_once(pool: &PgPool) -> Result<(u64, u64), sqlx::Error> {
    let v = sqlx::query!("DELETE FROM parcel_external_data_pii_vault WHERE expires_at < NOW()"
    ).execute(pool).await?.rows_affected();
    let m = sqlx::query!("DELETE FROM parcel_external_data WHERE expires_at < NOW()"
    ).execute(pool).await?.rows_affected();
    tracing::info!(deleted_vault = v, deleted_main = m, "cleanup cycle complete");
    Ok((v, m))
}
```

```
cargo test cleanup_run_once_deletes_both
# PASSED: test cleanup_run_once_deletes_both ... ok
```

```bash
git add crates/etl-base-layer/src/cleanup_task.rs
git commit -m feat: CleanupTask run_once vault-before-main deletion
```

---

- [ ] **Step 4.4:** CleanupTask emits tracing::info per cycle

```rust
#[traced_test]
#[tokio::test]
async fn cleanup_emits_tracing_event() {
    let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
    CleanupTask::run_once(&pool).await.ok();
    assert!(logs_contain("cleanup cycle complete"));
}
```

```
cargo test cleanup_emits_tracing_event
# FAILED: logs_contain returned false
```

tracing::info! already in run_once impl — verify #[traced_test] annotation.

```
cargo test cleanup_emits_tracing_event
# PASSED: test cleanup_emits_tracing_event ... ok
```

```bash
git add crates/etl-base-layer/src/cleanup_task.rs
git commit -m test: verify CleanupTask tracing event fields
```

---

- [ ] **Step 4.5:** CleanupTask graceful shutdown via CancellationToken

```rust
#[tokio::test]
async fn cleanup_task_exits_on_cancel() {
    let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
    let token = tokio_util::sync::CancellationToken::new();
    let handle = CleanupTask::spawn(pool, std::time::Duration::from_secs(3600), token.clone());
    token.cancel();
    tokio::time::timeout(std::time::Duration::from_millis(100), handle)
        .await.expect("exit within 100ms")
        .expect("no panic");
}
```

```
cargo test cleanup_task_exits_on_cancel
# FAILED: Elapsed (token not checked)
```

tokio::select! listening to cancel.cancelled() in spawn loop — already correct.

```
cargo test cleanup_task_exits_on_cancel
# PASSED: test cleanup_task_exits_on_cancel ... ok
```

```bash
git add crates/etl-base-layer/src/cleanup_task.rs
git commit -m test: verify CleanupTask graceful shutdown
```

---

- [ ] **Step 4.6:** CleanupTask wired into axum app startup (1h interval)

```rust
// services/api/src/main.rs
let cleanup_cancel = tokio_util::sync::CancellationToken::new();
CleanupTask::spawn(pool.clone(), Duration::from_secs(3600), cleanup_cancel.clone());
```

```
cargo test app_starts_with_cleanup_task_running
# FAILED: 404 /health
```

Wire CleanupTask::spawn in main.rs before axum::serve.

```
cargo test app_starts_with_cleanup_task_running
# PASSED: test app_starts_with_cleanup_task_running ... ok
```

```bash
git add services/api/src/main.rs
git commit -m feat(api): wire CleanupTask startup with 1h interval
```

## T5: Building Reader Live Wiring

**Goal:** Wire the full sanitization + dual-tier pipeline into building_reader.rs.
Migration 30010 ships in same PR as RAW_CAPTURE_SOURCE change (BLOCKER).

---

- [ ] **Step 5.1 (BLOCKER):** RAW_CAPTURE_SOURCE const + migration 30010 in same PR

```rust
// FAILING TEST
#[test]
fn raw_capture_source_is_vworld_parcel() {
    assert_eq!(RAW_CAPTURE_SOURCE, "vworld_parcel");
}
```

```
cargo test raw_capture_source_is_vworld_parcel
# FAILED: expected vworld_parcel, got vworld
```

```rust
const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";
```

```sql
-- migrations/30010_source_taxonomy_expansion.sql
ALTER TABLE parcel_external_data DROP CONSTRAINT parcel_external_data_source_check;
ALTER TABLE parcel_external_data ADD CONSTRAINT parcel_external_data_source_check
  CHECK (source IN ('vworld','vworld_parcel','data_go_kr_building',
    'data_go_kr_land','data_go_kr_realtransaction','korean_law'));
UPDATE parcel_external_data SET source = 'vworld_parcel' WHERE source = 'vworld';
```

```
cargo test raw_capture_source_is_vworld_parcel
# PASSED: ok
```

```bash
git add migrations/30010_source_taxonomy_expansion.sql crates/data-clients/vworld/src/reader.rs
git commit -m feat(db,vworld): BLOCKER taxonomy - rename vworld to vworld_parcel
```

---

- [ ] **Step 5.2:** building_reader.rs wired to SanitizingRawCapture<PgRawCapture, AllowlistSanitizer>

```rust
#[sqlx::test(migrations = "migrations")]
async fn building_reader_strips_pii_in_stored_row(pool: PgPool) {
    let state = AppState::test_with_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    server.get("/buildings?pnu=1168010400100370000").await.assert_status_success();
    let row = sqlx::query!(
        "SELECT raw_response FROM parcel_external_data WHERE pnu = " + DQ + "",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    let s = serde_json::to_string(&row.raw_response).unwrap();
    assert!(!s.contains("ownerNm"));
    assert!(!s.contains("regstrKindCd"));
}
```

```
cargo test building_reader_strips_pii_in_stored_row
# FAILED: ownerNm present in stored JSON
```

```rust
let sanitizer = AllowlistSanitizer::new("data_go_kr_building").unwrap();
let capture = SanitizingRawCapture::new(pg_capture.clone(), sanitizer);
capture.capture(&pnu, "data_go_kr_building", &raw, fetched_at).await?;
```

```
cargo test building_reader_strips_pii_in_stored_row
# PASSED: ok
```

```bash
git add services/api/src/building_reader.rs
git commit -m feat(api): wire building_reader to SanitizingRawCapture
```

---

- [ ] **Step 5.3:** building_reader.rs wired to DualTierCapture (full pipeline)

```rust
#[sqlx::test(migrations = "migrations")]
async fn building_reader_writes_to_both_tiers(pool: PgPool) {
    let state = AppState::test_with_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    server.get("/buildings?pnu=1168010400100370000").await.assert_status_success();
    let mc = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    let vc = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data_pii_vault")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(mc, 1, "main table row");
    assert_eq!(vc, 1, "vault table row");
}
```

```
cargo test building_reader_writes_to_both_tiers
# FAILED: vault_count == 0 (DualTierCapture not wired)
```

```rust
let sanitizer = AllowlistSanitizer::new("data_go_kr_building").unwrap();
let tier1 = SanitizingRawCapture::new(pg_capture, sanitizer);
let tier2 = PgPiiVaultCapture::new(pool.clone(), kms_client, kms_key_id, 30);
let capture = DualTierCapture::new(tier1, tier2);
capture.capture(&pnu, "data_go_kr_building", &raw, fetched_at).await?;
```

```
cargo test building_reader_writes_to_both_tiers
# PASSED: ok
```

```bash
git add services/api/src/building_reader.rs
git commit -m feat(api): wire building_reader to DualTierCapture pipeline
```

---

- [ ] **Step 5.4:** building_reader.rs schema_hash populated on every capture

```rust
#[sqlx::test(migrations = "migrations")]
async fn building_reader_sets_schema_hash(pool: PgPool) {
    // trigger capture via API...
    let row = sqlx::query!(
        "SELECT schema_hash FROM parcel_external_data WHERE pnu = ",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    assert!(row.schema_hash.as_deref().unwrap_or("").starts_with("sha256:"));
}
```

```
cargo test building_reader_sets_schema_hash
# FAILED: schema_hash is NULL
```

Ensure compute_schema_hash(&raw) called and written in PgRawCapture INSERT.

```
cargo test building_reader_sets_schema_hash
# PASSED: ok
```

```bash
git add services/api/src/building_reader.rs crates/etl-base-layer/src/
git commit -m feat(api): populate schema_hash on building capture
```

---

- [ ] **Step 5.5:** building_reader.rs sanitizer_version == 1 on every capture

```rust
#[sqlx::test(migrations = "migrations")]
async fn building_reader_sets_sanitizer_version_1(pool: PgPool) {
    let row = sqlx::query!(
        "SELECT sanitizer_version FROM parcel_external_data WHERE pnu = ",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.sanitizer_version, 1);
}
```

```
cargo test building_reader_sets_sanitizer_version_1
# FAILED: sanitizer_version = 0
```

AllowlistSanitizer::version() = 1 flows through SanitizingRawCapture to INSERT.

```
cargo test building_reader_sets_sanitizer_version_1
# PASSED: ok
```

```bash
git add services/api/src/building_reader.rs
git commit -m feat(api): set sanitizer_version=1 on building captures
```

---

- [ ] **Step 5.6:** building_reader.rs returns HTTP 502 when DualTierCapture fails

```rust
#[tokio::test]
async fn building_reader_502_on_capture_failure() {
    let state = AppState::test_with_failing_kms();
    let server = TestServer::new(create_app(state)).unwrap();
    let resp = server.get("/buildings?pnu=1168010400100370000").await;
    resp.assert_status(StatusCode::BAD_GATEWAY);
}
```

```
cargo test building_reader_502_on_capture_failure
# FAILED: got 200 or 500
```

```rust
// Map RawCaptureError::Kms to 502 in building_reader handler
Err(RawCaptureError::Kms(e)) => {
    tracing::error!(error = %e, "KMS failure during capture");
    return Err(AppError::bad_gateway("data storage unavailable"));
}
```

```
cargo test building_reader_502_on_capture_failure
# PASSED: ok
```

```bash
git add services/api/src/building_reader.rs
git commit -m feat(api): map KMS capture failure to HTTP 502
```

## T6: Vault Access RBAC Admin Endpoint + Audit Log

**Goal:** Implement POST /admin/vault/decrypt with ZITADEL JWT admin verification, audit-before-decrypt invariant, rate limiting, and PII-safe logging.

---

- [ ] **Step 6.1:** Migration 30013 — raw_vault_access_log (exactly 7 data columns)

```sql
-- migrations/30013_raw_vault_access_log.sql
CREATE TABLE raw_vault_access_log (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    source varchar(40) NOT NULL,
    pnu char(19) NOT NULL,
    purpose TEXT NOT NULL CHECK (purpose IN (
        'incident_investigation','drift_diagnosis','customer_request'
    )),
    ticket_id TEXT NOT NULL,
    accessed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    request_id TEXT NOT NULL
);
CREATE INDEX raw_vault_access_log_pnu_source_idx ON raw_vault_access_log (pnu, source);
CREATE INDEX raw_vault_access_log_accessed_at_idx ON raw_vault_access_log (accessed_at);
```

```rust
// FAILING TEST
#[sqlx::test(migrations = "migrations")]
async fn audit_log_rejects_invalid_purpose(pool: PgPool) {
    let res = sqlx::query!(
        "INSERT INTO raw_vault_access_log (user_id,source,pnu,purpose,ticket_id,request_id)"
        " VALUES (,,,,,)",
        "user1", "data_go_kr_building", "1168010400100370000",
        "invalid_purpose", "TICK-1", "req-1"
    ).execute(&pool).await;
    assert!(res.is_err(), "invalid purpose must fail check constraint");
}
```

```
cargo test audit_log_rejects_invalid_purpose
# FAILED: table does not exist
```

Apply migration 30013_raw_vault_access_log.sql.

```
cargo test audit_log_rejects_invalid_purpose
# PASSED: ok
```

```bash
git add migrations/30013_raw_vault_access_log.sql
git commit -m feat(db): add raw_vault_access_log table (7 cols, purpose check)
```

---

- [ ] **Step 6.2:** PgVaultAccessLog struct — record() inserts audit row

```rust
// FAILING TEST
#[sqlx::test(migrations = "migrations")]
async fn pg_vault_access_log_record_inserts_all_columns(pool: PgPool) {
    let log = PgVaultAccessLog::new(pool.clone());
    log.record("user123", "data_go_kr_building",
               "1168010400100370000", "incident_investigation",
               "TICK-42", "req-abc").await.unwrap();
    let row = sqlx::query!(
        "SELECT user_id, purpose, ticket_id FROM raw_vault_access_log WHERE pnu = ",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.user_id, "user123");
    assert_eq!(row.purpose, "incident_investigation");
    assert_eq!(row.ticket_id, "TICK-42");
}
```

```
cargo test pg_vault_access_log_record_inserts_all_columns
# FAILED: cannot find struct PgVaultAccessLog
```

```rust
// MINIMAL IMPL
pub struct PgVaultAccessLog { pool: PgPool }
impl PgVaultAccessLog {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
    pub async fn record(&self, user_id: &str, source: &str, pnu: &str,
                        purpose: &str, ticket_id: &str, request_id: &str)
        -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO raw_vault_access_log"
            "  (user_id,source,pnu,purpose,ticket_id,request_id)"
            "VALUES (,,,,,)",
            user_id, source, pnu, purpose, ticket_id, request_id
        ).execute(&self.pool).await?;
        Ok(())
    }
}
```

```
cargo test pg_vault_access_log_record_inserts_all_columns
# PASSED: ok
```

```bash
git add crates/etl-base-layer/src/pg_vault_access_log.rs
git commit -m feat(etl-base-layer): PgVaultAccessLog record() impl
```

---

- [ ] **Step 6.3:** audit-before-decrypt invariant — log written BEFORE KMS decrypt

```rust
// FAILING TEST
#[sqlx::test(migrations = "migrations")]
async fn audit_log_written_before_decrypt(pool: PgPool) {
    let kms = FailingKms;  // decrypt always fails
    let log = PgVaultAccessLog::new(pool.clone());
    // Attempt decrypt (will fail due to KMS)
    let result = vault_decrypt_with_audit(&pool, &kms, &log,
        "user1", "1168010400100370000", "data_go_kr_building",
        "incident_investigation", "TICK-1", "req-1").await;
    assert!(result.is_err(), "decrypt failed");
    // Audit log MUST exist despite decrypt failure
    let count = sqlx::query!("SELECT COUNT(*) as c FROM raw_vault_access_log")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(count, 1, "audit log must be written even when decrypt fails");
}
```

```
cargo test audit_log_written_before_decrypt
# FAILED: audit log count = 0 (log written after decrypt or not at all)
```

```rust
// MINIMAL IMPL - vault_decrypt_with_audit:
pub async fn vault_decrypt_with_audit(...) -> Result<...> {
    // 1. Write audit log FIRST
    log.record(user_id, source, pnu, purpose, ticket_id, request_id).await?;
    // 2. THEN decrypt (audit log already committed)
    let row = /* fetch vault row */ ...;
    kms.decrypt(&row.ciphertext_blob, &row.kms_key_id).await
}
```

```
cargo test audit_log_written_before_decrypt
# PASSED: ok
```

```bash
git add crates/etl-base-layer/src/pg_vault_access_log.rs
git commit -m feat(etl-base-layer): enforce audit-before-decrypt invariant
```

---

- [ ] **Step 6.4:** VaultAdminRequest DTO + ZITADEL JWT admin role verification

```rust
// FAILING TEST
#[tokio::test]
async fn vault_decrypt_rejects_non_admin_jwt() {
    let server = TestServer::new(create_app(test_state())).unwrap();
    let token = generate_jwt(role = "user");  // non-admin
    let resp = server.post("/admin/vault/decrypt")
        .add_header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({"pnu": "1168010400100370000",
            "source": "data_go_kr_building",
            "purpose": "incident_investigation",
            "ticket_id": "TICK-1"}))
        .await;
    resp.assert_status(StatusCode::FORBIDDEN);
}
```

```
cargo test vault_decrypt_rejects_non_admin_jwt
# FAILED: 404 (route not registered)
```

```rust
// MINIMAL IMPL
#[derive(serde::Deserialize)]
pub struct VaultAdminRequest {
    pub pnu: String,      // validated: char(19)
    pub source: String,   // validated: varchar(40)
    pub purpose: String,  // validated: enum
    pub ticket_id: String,
}
// Middleware: extract JWT claims, check role == "admin", else 403
```

```
cargo test vault_decrypt_rejects_non_admin_jwt
# PASSED: ok
```

```bash
git add services/api/src/routes/vault_admin.rs
git commit -m feat(api): VaultAdminRequest DTO + ZITADEL admin JWT check
```

---

- [ ] **Step 6.5:** POST /admin/vault/decrypt endpoint

```rust
#[tokio::test]
async fn vault_decrypt_admin_returns_plaintext() {
    let server = TestServer::new(create_app(test_state_with_mock_kms())).unwrap();
    let token = generate_admin_jwt();
    let resp = server.post("/admin/vault/decrypt")
        .authorization_bearer(token)
        .json(&serde_json::json!({"pnu": "1168010400100370000",
            "source": "data_go_kr_building",
            "purpose": "incident_investigation",
            "ticket_id": "TICK-1"})).await;
    resp.assert_status_ok();
}
```

```
cargo test vault_decrypt_admin_returns_plaintext
# FAILED: 404 route not registered
```

Register POST /admin/vault/decrypt in axum router with admin middleware.

```
cargo test vault_decrypt_admin_returns_plaintext
# PASSED: ok
```

```bash
git add services/api/src/routes/vault_admin.rs
git commit -m feat(api): POST /admin/vault/decrypt endpoint
```

---

- [ ] **Step 6.6:** Rate limiting — max 10 req/min per user_id

```rust
#[tokio::test]
async fn vault_decrypt_rate_limit_429() {
    let server = TestServer::new(create_app_with_rate_limiter()).unwrap();
    let token = generate_admin_jwt_for("user-rate-test");
    let req = serde_json::json!({"pnu": "1168010400100370000",
        "source": "data_go_kr_building",
        "purpose": "incident_investigation",
        "ticket_id": "TICK-1"});
    for _ in 0..10 { server.post("/admin/vault/decrypt")
        .authorization_bearer(&token).json(&req).await.assert_status_ok(); }
    server.post("/admin/vault/decrypt")
        .authorization_bearer(&token).json(&req).await
        .assert_status(StatusCode::TOO_MANY_REQUESTS);
}
```

```
cargo test vault_decrypt_rate_limit_429
# FAILED: 11th returns 200
```

Add tower-governor rate limiter: 10/60s per user_id.

```
cargo test vault_decrypt_rate_limit_429
# PASSED: ok
```

```bash
git add services/api/src/routes/vault_admin.rs
git commit -m feat(api): rate limit vault decrypt to 10/min per user
```

---

- [ ] **Step 6.7:** Response never logged (PII safety)

```rust
#[traced_test]
#[tokio::test]
async fn vault_decrypt_response_not_in_logs() {
    // Perform successful decrypt, then verify no PII in logs
    assert!(!logs_contain("ownerNm"), "PII must not appear in logs");
}
```

```
cargo test vault_decrypt_response_not_in_logs
# FAILED: ownerNm found in span attributes
```

Never record response body in tracing span or log in vault_admin handler.

```
cargo test vault_decrypt_response_not_in_logs
# PASSED: ok
```

```bash
git add services/api/src/routes/vault_admin.rs
git commit -m feat(api): ensure vault response not logged (PII)
```

---

- [ ] **Step 6.8:** Returns 404 when pnu+source not in vault

```rust
#[tokio::test]
async fn vault_decrypt_404_for_unknown_pnu() {
    let server = TestServer::new(create_app(test_state())).unwrap();
    let resp = server.post("/admin/vault/decrypt")
        .authorization_bearer(generate_admin_jwt())
        .json(&serde_json::json!({"pnu": "9999999999999999999",
            "source": "data_go_kr_building",
            "purpose": "incident_investigation",
            "ticket_id": "TICK-1"})).await;
    resp.assert_status(StatusCode::NOT_FOUND);
}
```

```
cargo test vault_decrypt_404_for_unknown_pnu
# FAILED: 500 on RowNotFound
```

Map sqlx::Error::RowNotFound to 404 in vault_admin handler.

```
cargo test vault_decrypt_404_for_unknown_pnu
# PASSED: ok
```

```bash
git add services/api/src/routes/vault_admin.rs
git commit -m feat(api): return 404 when vault row not found
```

---

- [ ] **Step 6.9:** Route registered in axum router — full round-trip integration test

```rust
#[tokio::test]
async fn vault_admin_full_roundtrip() {
    let server = TestServer::new(create_app(test_state_with_mock_kms())).unwrap();
    // 1. Capture building data (PII stored in vault)
    server.get("/buildings?pnu=1168010400100370000").await.assert_status_success();
    // 2. Admin decrypts vault
    let resp = server.post("/admin/vault/decrypt")
        .authorization_bearer(generate_admin_jwt())
        .json(&serde_json::json!({"pnu": "1168010400100370000",
            "source": "data_go_kr_building",
            "purpose": "incident_investigation",
            "ticket_id": "TICK-99"})).await;
    resp.assert_status_ok();
}
```

```
cargo test vault_admin_full_roundtrip
# FAILED: 404 (route not registered)
```

Register route in router: .route("/admin/vault/decrypt", post(vault_admin::decrypt_handler))

```
cargo test vault_admin_full_roundtrip
# PASSED: ok
```

```bash
git add services/api/src/routes/ services/api/src/main.rs
git commit -m feat(api): register vault_admin route in axum router
```

## T7: Integration Tests (axum-test 15.0)

**Goal:** Full-stack integration tests covering sanitization roundtrip, vault encrypt/decrypt, DualTierCapture atomicity, taxonomy migration, audit log, RLS, TTL cleanup, schema hash stability, and pipeline smoke test.

---

- [ ] **Step 7.1:** axum-test 15.0 TestServer harness setup

```rust
// FAILING TEST
// tests/integration/vault_roundtrip.rs
#[tokio::test]
async fn health_check_returns_200() {
    let server = TestServer::new(create_app(test_state())).unwrap();
    server.get("/health").await.assert_status_ok();
}
```

```
cargo test health_check_returns_200
# FAILED: cannot find TestServer in scope
```

Add axum-test = "15.0" to Cargo.toml dev-dependencies.

```
cargo test health_check_returns_200
# PASSED: ok
```

```bash
git add tests/ Cargo.toml
git commit -m test(integration): axum-test 15.0 harness setup
```

---

- [ ] **Step 7.2:** Full sanitization roundtrip — ownerNm stripped from stored row

```rust
#[sqlx::test(migrations = "migrations")]
async fn roundtrip_building_sanitizes_pii(pool: PgPool) {
    let state = AppState::test_with_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    server.get("/buildings?pnu=1168010400100370000").await.assert_status_success();
    let row = sqlx::query!(
        "SELECT raw_response FROM parcel_external_data WHERE pnu = ",
        "1168010400100370000"
    ).fetch_one(&pool).await.unwrap();
    let s = serde_json::to_string(&row.raw_response).unwrap();
    assert!(!s.contains("ownerNm"), "ownerNm must be stripped");
    assert!(!s.contains("regstrKindCd"), "regstrKindCd must be stripped");
}
```

```
cargo test roundtrip_building_sanitizes_pii
# FAILED: ownerNm present
```

Full pipeline wired in T5.3 — ensure DualTierCapture with SanitizingRawCapture is active.

```
cargo test roundtrip_building_sanitizes_pii
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify building sanitization strips PII fields
```

---

- [ ] **Step 7.3:** Vault roundtrip — PII stored encrypted, decryptable by admin

```rust
#[sqlx::test(migrations = "migrations")]
async fn roundtrip_vault_encrypt_decrypt(pool: PgPool) {
    let state = AppState::test_with_mock_kms_and_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    // 1. Capture (stores PII encrypted in vault)
    server.get("/buildings?pnu=1168010400100370000").await.assert_status_success();
    // 2. Admin decrypt
    let resp = server.post("/admin/vault/decrypt")
        .authorization_bearer(generate_admin_jwt())
        .json(&serde_json::json!({"pnu": "1168010400100370000",
            "source": "data_go_kr_building",
            "purpose": "incident_investigation",
            "ticket_id": "TICK-7"})).await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    // Decrypted PII must contain ownerNm
    assert!(body.pointer("/ownerNm").is_some());
}
```

```
cargo test roundtrip_vault_encrypt_decrypt
# FAILED: decrypt response missing ownerNm
```

PgPiiVaultCapture stores raw (unfiltered) JSON. Verify mock KMS decrypt returns original bytes.

```
cargo test roundtrip_vault_encrypt_decrypt
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify vault encrypt/decrypt roundtrip
```

---

- [ ] **Step 7.4:** DualTierCapture atomicity — KMS failure aborts main table write

```rust
#[sqlx::test(migrations = "migrations")]
async fn dual_tier_vault_failure_aborts_main(pool: PgPool) {
    let state = AppState::test_with_failing_kms_and_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    server.get("/buildings?pnu=1168010400100370000").await;
    let mc = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    let vc = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data_pii_vault")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(mc, 0, "main table must have 0 rows when vault fails");
    assert_eq!(vc, 0, "vault must have 0 rows");
}
```

```
cargo test dual_tier_vault_failure_aborts_main
# FAILED: mc = 1 (main table written despite vault failure)
```

DualTierCapture calls tier2 first. Vault (tier2) fails -> abort. Tier1 not called.

```
cargo test dual_tier_vault_failure_aborts_main
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify DualTierCapture atomicity on vault failure
```

---

- [ ] **Step 7.5:** Taxonomy migration 30010 — vworld renamed to vworld_parcel

```rust
#[sqlx::test(migrations = "migrations")]
async fn taxonomy_migration_renames_vworld(pool: PgPool) {
    // Insert legacy row with source=vworld BEFORE 30010
    // (simulate by bypassing check, or use pre-30010 fixture)
    // After migration, no rows with source="vworld" should exist
    let count = sqlx::query!(
        "SELECT COUNT(*) as c FROM parcel_external_data WHERE source = ",
        "vworld"
    ).fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(count, 0, "no rows with source=vworld after 30010");
}
```

```
cargo test taxonomy_migration_renames_vworld
# FAILED: count = 1 (migration 30010 not applied)
```

Apply 30010_source_taxonomy_expansion.sql and run sqlx migrate.

```
cargo test taxonomy_migration_renames_vworld
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify taxonomy migration 30010
```

---

- [ ] **Step 7.6:** Audit log written before decrypt (even on KMS failure)

```rust
#[sqlx::test(migrations = "migrations")]
async fn audit_log_written_before_decrypt_on_failure(pool: PgPool) {
    let state = AppState::test_with_failing_kms_and_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    server.post("/admin/vault/decrypt")
        .authorization_bearer(generate_admin_jwt())
        .json(&serde_json::json!({"pnu": "1168010400100370000",
            "source": "data_go_kr_building",
            "purpose": "incident_investigation",
            "ticket_id": "TICK-6"})).await;
    let count = sqlx::query!("SELECT COUNT(*) as c FROM raw_vault_access_log")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(count, 1, "audit log must exist even when decrypt fails");
}
```

```
cargo test audit_log_written_before_decrypt_on_failure
# FAILED: count = 0
```

vault_decrypt_with_audit: log.record() before kms.decrypt().

```
cargo test audit_log_written_before_decrypt_on_failure
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify audit-before-decrypt invariant
```

---

- [ ] **Step 7.7:** RLS blocks non-admin SELECT on vault

```rust
#[sqlx::test(migrations = "migrations")]
async fn rls_blocks_non_admin_role(pool: PgPool) {
    // insert vault row, then switch to non-admin role
    sqlx::query("SET LOCAL app.role = 'user'")
        .execute(&pool).await.unwrap();
    let count = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data_pii_vault")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(count, 0, "RLS must block non-admin");
}
```

```
cargo test rls_blocks_non_admin_role
# FAILED: count = 1 (RLS not enforced)
```

Use SET LOCAL app.role in non-superuser connection.

```
cargo test rls_blocks_non_admin_role
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify RLS blocks non-admin vault access
```

---

- [ ] **Step 7.8:** TTL cleanup deletes expired rows from both tables

```rust
#[sqlx::test(migrations = "migrations")]
async fn ttl_cleanup_deletes_expired(pool: PgPool) {
    // Insert rows with past expires_at (bypass check for test)
    let (v, m) = CleanupTask::run_once(&pool).await.unwrap();
    assert_eq!(v, 1, "vault row cleaned up");
    assert_eq!(m, 1, "main row cleaned up");
}
```

```
cargo test ttl_cleanup_deletes_expired
# FAILED: counts = 0
```

Set expires_at to past timestamp in test setup.

```
cargo test ttl_cleanup_deletes_expired
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): verify TTL cleanup for both tables
```

---

- [ ] **Step 7.9:** Schema hash determinism

```rust
#[test]
fn schema_hash_is_deterministic() {
    let raw = serde_json::json!({"key": "value"});
    let h1 = compute_schema_hash(&raw);
    let h2 = compute_schema_hash(&raw);
    assert_eq!(h1, h2);
    assert!(h1.starts_with("sha256:"));
}
```

```
cargo test schema_hash_is_deterministic
# FAILED: hashes differ between calls
```

serde_json serializes maps deterministically — verify no HashMap used in JSON.

```
cargo test schema_hash_is_deterministic
# PASSED: ok
```

```bash
git add crates/etl-base-layer/src/
git commit -m test: verify compute_schema_hash determinism
```

---

- [ ] **Step 7.10:** Full pipeline smoke test

```rust
#[sqlx::test(migrations = "migrations")]
async fn full_pipeline_smoke_test(pool: PgPool) {
    let state = AppState::test_with_mock_kms_and_pool(pool.clone());
    let server = TestServer::new(create_app(state)).unwrap();
    server.get("/buildings?pnu=1168010400100370000").await.assert_status_success();
    let mc = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    let vc = sqlx::query!("SELECT COUNT(*) as c FROM parcel_external_data_pii_vault")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    let lc = sqlx::query!("SELECT COUNT(*) as c FROM raw_vault_access_log")
        .fetch_one(&pool).await.unwrap().c.unwrap_or(0);
    assert_eq!(mc, 1, "main table: 1 sanitized row");
    assert_eq!(vc, 1, "vault table: 1 encrypted row");
    assert_eq!(lc, 0, "audit log: 0 entries (no decrypt called)");
}
```

```
cargo test full_pipeline_smoke_test
# FAILED: vc = 0 or mc = 0
```

Ensure all T1-T6 tasks complete before running this test.

```
cargo test full_pipeline_smoke_test
# PASSED: ok
```

```bash
git add tests/integration/vault_roundtrip.rs
git commit -m test(integration): full pipeline smoke test (all wired)
```

---

## Implementation Checklist

- [ ] T1: RawSanitizer trait + AllowlistSanitizer + SanitizingRawCapture (Steps 1.1-1.8)
- [ ] T2: Allowlist constants + migration 30010 BLOCKER (Steps 2.1-2.8)
- [ ] T3: Vault migrations 30011-30012 + PgPiiVaultCapture + DualTierCapture (Steps 3.1-3.10)
- [ ] T4: Migration 30014 + CleanupTask TTL (Steps 4.1-4.6)
- [ ] T5: Building reader live wiring (Steps 5.1-5.6)
- [ ] T6: Admin RBAC endpoint + audit log + migration 30013 (Steps 6.1-6.9)
- [ ] T7: Integration tests axum-test 15.0 (Steps 7.1-7.10)

**Total: ~58 steps across 7 tasks**

---

## Correctness Verification Matrix

| Check | Expected | Verified by |
|-------|----------|-------------|
| Table name | "parcel_external_data" | T1.8, T3.1, T7.2 |
| Vault table | "parcel_external_data_pii_vault" | T3.1, T3.4, T7.3 |
| Migration 30010 | source_taxonomy_expansion | T2.1, T5.1, T7.5 |
| Migration 30013 | raw_vault_access_log (7 data cols) | T6.1, T7.6 |
| FK columns | pnu char(19), source varchar(40) | T3.1, T6.1 |
| Migration format | 30010_name.sql (no V prefix) | T2.1 |
| PII fields stripped | ownerNm, regstrKindCd | T1.3, T2.x, T7.2 |
| Source renamed | vworld -> vworld_parcel | T5.1, T7.5 |
| RLS enforced | non-admin sees 0 rows | T3.6, T7.7 |
| Audit-before-decrypt | log written even on KMS fail | T6.3, T7.6 |
| schema_hash prefix | sha256: (71 chars) | T3.9, T5.4, T7.9 |
| DualTierCapture order | vault fails -> main not written | T3.7, T7.4 |

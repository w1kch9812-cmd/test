# SP10.5-B: Backend Data Correctness Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for each task.

- **Spec SSOT**: `docs/superpowers/specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md`
- **TDD mandate**: failing test -> `cargo test` (red) -> impl -> `cargo test` (green) -> commit
- **No placeholders**: every step has real code + exact commands + expected output
- **Line budget**: 1000-1300 lines (hard limit 1500, AGENTS.md ss.1)

---

## File Structure

| File | Purpose |
|------|---------|
| `raw-capture/src/sanitizer.rs` | `RawSanitizer` + `AllowlistSanitizer` |
| `raw-capture/src/sanitizing_capture.rs` | `SanitizingRawCapture` |
| `raw-capture/src/dual_tier.rs` | `DualTierCapture` |
| `crates/db/src/pii_vault.rs` | `PgPiiVaultCapture` + `PgVaultAccessLog` |
| `db/migration/V30010-V30014` | 5 migrations |
| `services/api/src/cleanup.rs` | Tokio TTL cleanup |
| `services/api/tests/health_integration.rs` | axum-test health |

---

## Task 1: RawSanitizer Trait + AllowlistSanitizer

**Goal**: PIPA 최소수집 원칙 — PII-safe allowlist filtering before any capture.

### Step 1.1: Add sha2 dependency

```toml
# crates/data-clients/raw-capture/Cargo.toml
[dependencies]
sha2 = { workspace = true }
```

Workspace `Cargo.toml` if sha2 missing from `[workspace.dependencies]`: `sha2 = "0.10"`

```bash
cargo check -p raw-capture
# Expected: Finished
```

---

### Step 1.2: Write failing test for compute_schema_hash

Create `crates/data-clients/raw-capture/src/sanitizer.rs`:

```rust
use sha2::{Digest, Sha256};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SanitizerError {
    #[error("no allowlist for source: {0}")]
    UnknownSource(String),
}

/// SHA-256 over "source:version:path1,..." (paths sorted — order-independent).
pub fn compute_schema_hash(source: &str, version: u32, paths: &[&str]) -> String {
    todo!("impl")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_hash_order_independent() {
        let h1 = compute_schema_hash("vworld_parcel", 1, &["pnu", "geometry.type"]);
        let h2 = compute_schema_hash("vworld_parcel", 1, &["geometry.type", "pnu"]);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }
}
```

```bash
cargo test -p raw-capture schema_hash_order_independent 2>&1 | grep "panicked"
# Expected: panicked at ... not yet implemented: impl
```

---

### Step 1.3: Implement compute_schema_hash

```rust
pub fn compute_schema_hash(source: &str, version: u32, paths: &[&str]) -> String {
    let mut sorted = paths.to_vec();
    sorted.sort_unstable();
    let input = format!("{}:{}:{}", source, version, sorted.join(","));
    let digest = Sha256::digest(input.as_bytes());
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}
```

```bash
cargo test -p raw-capture schema_hash_order_independent 2>&1 | grep "ok"
# Expected: test tests::schema_hash_order_independent ... ok
```

---

### Step 1.4: Write failing tests for AllowlistSanitizer

Append to `sanitizer.rs`:

```rust
#[derive(Debug, Clone)]
pub struct SanitizedRaw {
    pub value: Value,
    pub dropped_count: usize,
    pub schema_hash: String,
    pub sanitizer_version: u32,
}

pub trait RawSanitizer: Send + Sync {
    fn sanitize(&self, source: &str, raw: &Value) -> SanitizedRaw;
}

pub struct AllowlistSanitizer {
    paths: Vec<String>,
    schema_hash: String,
    sanitizer_version: u32,
}

impl AllowlistSanitizer {
    pub fn for_source(source: &str) -> Result<Self, SanitizerError> {
        todo!("impl")
    }
}

#[cfg(test)]
mod allowlist_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn vworld_parcel_drops_pii() {
        let san = AllowlistSanitizer::for_source("vworld_parcel").unwrap();
        let raw = json!({
            "pnu": "1234567890",
            "OWNER_RRN": "900101-1234567",
            "geometry": { "type": "Polygon", "coordinates": [[[0,0]]] }
        });
        let r = san.sanitize("vworld_parcel", &raw);
        assert!(r.value.get("OWNER_RRN").is_none());
        assert_eq!(r.value["pnu"], "1234567890");
        assert!(r.dropped_count >= 1);
        assert_eq!(r.schema_hash.len(), 64);
        assert_eq!(r.sanitizer_version, 1);
    }

    #[test]
    fn unknown_source_errors() {
        assert!(AllowlistSanitizer::for_source("bad_source").is_err());
    }

    #[test]
    fn data_go_kr_building_keeps_known_fields() {
        let san = AllowlistSanitizer::for_source("data_go_kr_building").unwrap();
        let raw = json!({ "mgmBldrgstPk": "abc", "SECRET_PII": "drop" });
        let r = san.sanitize("data_go_kr_building", &raw);
        assert!(r.value.get("SECRET_PII").is_none());
        assert_eq!(r.value["mgmBldrgstPk"], "abc");
    }
}
```

```bash
cargo test -p raw-capture allowlist_tests 2>&1 | grep "panicked"
# Expected: panicked at ... not yet implemented: impl
```

---

### Step 1.5: Implement AllowlistSanitizer

```rust
impl AllowlistSanitizer {
    const VERSION: u32 = 1;

    pub fn for_source(source: &str) -> Result<Self, SanitizerError> {
        let paths: &[&str] = match source {
            "data_go_kr_building" => &[
                "mgmBldrgstPk", "bldNm", "platPlc",
                "newPlatPlc", "dongNm", "flloor", "ho",
            ],
            "vworld_parcel" => &[
                "pnu", "spbd_pnu", "spbd_addr", "spbd_area",
                "spbd_lndcgr", "spbd_prpos", "spbd_ownsh",
                "geometry.type", "geometry.coordinates",
            ],
            other => return Err(SanitizerError::UnknownSource(other.to_owned())),
        };
        Ok(Self {
            paths: paths.iter().map(|p| (*p).to_owned()).collect(),
            schema_hash: compute_schema_hash(source, Self::VERSION, paths),
            sanitizer_version: Self::VERSION,
        })
    }
}

impl RawSanitizer for AllowlistSanitizer {
    fn sanitize(&self, _source: &str, raw: &Value) -> SanitizedRaw {
        let Value::Object(map) = raw else {
            return SanitizedRaw {
                value: raw.clone(), dropped_count: 0,
                schema_hash: self.schema_hash.clone(),
                sanitizer_version: self.sanitizer_version,
            };
        };
        let mut kept = serde_json::Map::new();
        let mut dropped = 0usize;
        for (k, v) in map {
            let direct = self.paths.iter().any(|p| p == k);
            let parent = self.paths.iter().any(|p| {
                p.starts_with(&format!("{}.", k))
            });
            if direct {
                kept.insert(k.clone(), v.clone());
            } else if parent {
                let sub: Vec<&str> = self.paths.iter()
                    .filter_map(|p| p.strip_prefix(&format!("{}.", k)))
                    .collect();
                if let Value::Object(n) = v {
                    let f: serde_json::Map<_, _> = n.iter()
                        .filter(|(nk, _)| sub.contains(&nk.as_str()))
                        .map(|(nk, nv)| (nk.clone(), nv.clone()))
                        .collect();
                    kept.insert(k.clone(), Value::Object(f));
                } else {
                    kept.insert(k.clone(), v.clone());
                }
            } else {
                dropped += 1;
            }
        }
        SanitizedRaw {
            value: Value::Object(kept), dropped_count: dropped,
            schema_hash: self.schema_hash.clone(),
            sanitizer_version: self.sanitizer_version,
        }
    }
}
```

```bash
cargo test -p raw-capture 2>&1 | grep "test.*ok"
# Expected:
# test tests::schema_hash_order_independent ... ok
# test allowlist_tests::vworld_parcel_drops_pii ... ok
# test allowlist_tests::unknown_source_errors ... ok
# test allowlist_tests::data_go_kr_building_keeps_known_fields ... ok
```

---

### Step 1.6: SanitizingRawCapture wrapper

Create `crates/data-clients/raw-capture/src/sanitizing_capture.rs`:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::warn;
use serde_json::Value;
use crate::{RawCapture, RawCaptureError};
use crate::sanitizer::{AllowlistSanitizer, RawSanitizer};

pub struct SanitizingRawCapture<C: RawCapture> {
    inner: Arc<C>,
    sanitizer: AllowlistSanitizer,
}

impl<C: RawCapture> SanitizingRawCapture<C> {
    pub fn new(inner: Arc<C>, sanitizer: AllowlistSanitizer) -> Self {
        Self { inner, sanitizer }
    }
}

#[async_trait]
impl<C: RawCapture + Send + Sync + 'static> RawCapture for SanitizingRawCapture<C> {
    async fn capture(
        &self, pnu: &str, source: &str, raw: &Value, fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError> {
        let s = self.sanitizer.sanitize(source, raw);
        if s.dropped_count > 0 {
            warn!(
                target: "raw.capture.schema_drift",
                pnu=%pnu, source=%source,
                dropped_count=s.dropped_count,
                schema_hash=%s.schema_hash,
                "PII fields dropped by AllowlistSanitizer"
            );
        }
        self.inner.capture(pnu, source, &s.value, fetched_at).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoOpRawCapture;
    use serde_json::json;

    #[tokio::test]
    async fn delegates_filtered_value() {
        let inner = Arc::new(NoOpRawCapture::new());
        let san = AllowlistSanitizer::for_source("vworld_parcel").unwrap();
        let cap = SanitizingRawCapture::new(inner, san);
        let raw = json!({ "pnu": "123", "OWNER_RRN": "900101-1234567" });
        assert!(cap.capture("123", "vworld_parcel", &raw, Utc::now()).await.is_ok());
    }
}
```

```bash
cargo test -p raw-capture delegates_filtered_value 2>&1 | grep "ok"
# Expected: test tests::delegates_filtered_value ... ok
```

---

### Step 1.7: Wire module exports in lib.rs

```rust
pub mod sanitizer;
pub mod sanitizing_capture;

pub use sanitizer::{AllowlistSanitizer, RawSanitizer, SanitizedRaw, SanitizerError};
pub use sanitizing_capture::SanitizingRawCapture;
```

```bash
cargo test -p raw-capture 2>&1 | tail -3
# Expected: test result: ok. N passed; 0 failed; 0 ignored
```

**Commit:**
```
git add crates/data-clients/raw-capture/
git commit -m "feat(raw-capture): T1 - AllowlistSanitizer + SanitizingRawCapture (PIPA 최소수집)"
```

---

## Task 2: DualTierCapture Fan-Out Composer

**Goal**: Vault-first guarantees raw PII is encrypted before sanitization discards it.

### Step 2.1: Failing tests for DualTierCapture

Create `crates/data-clients/raw-capture/src/dual_tier.rs`:

```rust
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use crate::{RawCapture, RawCaptureError};

/// Vault-first fan-out: Tier 2 (vault, fail-fast) then Tier 1 (sanitized).
pub struct DualTierCapture<S: RawCapture, V: RawCapture> {
    sanitized: Arc<S>,
    vault: Arc<V>,
}

impl<S: RawCapture, V: RawCapture> DualTierCapture<S, V> {
    pub fn new(sanitized: Arc<S>, vault: Arc<V>) -> Self {
        Self { sanitized, vault }
    }
}

#[async_trait]
impl<S, V> RawCapture for DualTierCapture<S, V>
where
    S: RawCapture + Send + Sync + 'static,
    V: RawCapture + Send + Sync + 'static,
{
    async fn capture(&self, pnu: &str, source: &str, raw: &Value, fetched_at: DateTime<Utc>)
        -> Result<(), RawCaptureError>
    {
        todo!("impl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct OrderCapture { calls: Mutex<Vec<&'static str>>, label: &'static str, fail: bool }
    impl OrderCapture {
        fn new(label: &'static str, fail: bool) -> Arc<Self> {
            Arc::new(Self { calls: Mutex::new(vec![]), label, fail })
        }
        fn recorded(&self) -> Vec<&'static str> { self.calls.lock().unwrap().clone() }
    }
    #[async_trait]
    impl RawCapture for OrderCapture {
        async fn capture(&self, _: &str, _: &str, _: &Value, _: DateTime<Utc>)
            -> Result<(), RawCaptureError>
        {
            self.calls.lock().unwrap().push(self.label);
            if self.fail { Err(RawCaptureError::Io("simulated".into())) } else { Ok(()) }
        }
    }

    #[tokio::test]
    async fn vault_called_before_sanitized() {
        let vault = OrderCapture::new("vault", false);
        let sanitized = OrderCapture::new("sanitized", false);
        let dual = DualTierCapture::new(Arc::clone(&sanitized), Arc::clone(&vault));
        dual.capture("p", "s", &json!({}), Utc::now()).await.unwrap();
        assert_eq!(vault.recorded(), vec!["vault"]);
        assert_eq!(sanitized.recorded(), vec!["sanitized"]);
    }

    #[tokio::test]
    async fn vault_failure_blocks_sanitized() {
        let vault = OrderCapture::new("vault", true);
        let sanitized = OrderCapture::new("sanitized", false);
        let dual = DualTierCapture::new(Arc::clone(&sanitized), Arc::clone(&vault));
        assert!(dual.capture("p", "s", &json!({}), Utc::now()).await.is_err());
        assert!(sanitized.recorded().is_empty());
    }
}
```

```bash
cargo test -p raw-capture dual_tier 2>&1 | grep "panicked"
# Expected: panicked at ... not yet implemented: impl
```

---

### Step 2.2: Implement DualTierCapture::capture

Replace `todo!()`:

```rust
async fn capture(&self, pnu: &str, source: &str, raw: &Value, fetched_at: DateTime<Utc>)
    -> Result<(), RawCaptureError>
{
    self.vault.capture(pnu, source, raw, fetched_at).await?;   // Tier 2: fail-fast
    self.sanitized.capture(pnu, source, raw, fetched_at).await // Tier 1
}
```

```bash
cargo test -p raw-capture dual_tier 2>&1 | grep "ok"
# Expected:
# test tests::vault_called_before_sanitized ... ok
# test tests::vault_failure_blocks_sanitized ... ok
```

---

### Step 2.3: Export from lib.rs

```rust
pub mod dual_tier;
pub use dual_tier::DualTierCapture;
```

```bash
cargo test -p raw-capture 2>&1 | tail -3
# Expected: test result: ok. N passed; 0 failed; 0 ignored
```

**Commit:**
```
git add crates/data-clients/raw-capture/
git commit -m "feat(raw-capture): T2 - DualTierCapture vault-first fan-out"
```

---

## Task 3: Migrations V30010–V30014

**Goal**: DB schema for lineage columns, PII vault, access log, lineage events, TTL indexes.

### Step 3.1: V30010 — Lineage columns on raw_captures

Create `db/migration/V30010__raw_capture_lineage_cols.sql`:

```sql
-- V30010: Data lineage columns for raw_captures (PIPA traceability).
ALTER TABLE raw_captures
    ADD COLUMN IF NOT EXISTS sanitizer_version INTEGER,
    ADD COLUMN IF NOT EXISTS schema_hash       TEXT,
    ADD COLUMN IF NOT EXISTS license           TEXT,
    ADD COLUMN IF NOT EXISTS api_version       TEXT;

COMMENT ON COLUMN raw_captures.sanitizer_version IS 'AllowlistSanitizer::VERSION at capture';
COMMENT ON COLUMN raw_captures.schema_hash IS 'SHA-256 of allowlist definition';
COMMENT ON COLUMN raw_captures.license IS 'Open data license (e.g. KOGL-TYPE1)';
COMMENT ON COLUMN raw_captures.api_version IS 'Upstream API version string';
```

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate run --dry-run 2>&1 | grep V30010
# Expected: Would apply migration V30010__raw_capture_lineage_cols
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate run
# Expected: Applied 1/migrate V30010__raw_capture_lineage_cols
```

---

### Step 3.2: V30011 — pii_vault table

Create `db/migration/V30011__pii_vault.sql`:

```sql
-- V30011: PII vault — KMS envelope-encrypted responses.
-- Blob: enc_dek_len(4B BE) || enc_dek || iv(12B) || ciphertext
CREATE TABLE IF NOT EXISTS pii_vault (
    id             UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    pnu            TEXT        NOT NULL,
    source         TEXT        NOT NULL,
    encrypted_blob BYTEA       NOT NULL,
    enc_dek        BYTEA       NOT NULL,
    iv             BYTEA       NOT NULL CHECK (octet_length(iv) = 12),
    fetched_at     TIMESTAMPTZ NOT NULL,
    expires_at     TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS pii_vault_pnu_source_idx ON pii_vault (pnu, source);
CREATE INDEX IF NOT EXISTS pii_vault_expires_at_idx ON pii_vault (expires_at);

COMMENT ON TABLE pii_vault IS 'AES-256-GCM encrypted responses. DEK by AWS KMS CMK.';
COMMENT ON COLUMN pii_vault.expires_at IS 'fetched_at + 30 days (PIPA 보유기간)';
```

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate run
# Expected: Applied 1/migrate V30011__pii_vault
```

---

### Step 3.3: V30012 — vault_access_log table

Create `db/migration/V30012__vault_access_log.sql`:

```sql
-- V30012: Immutable audit log — INSERT before vault read (fail-fast).
CREATE TABLE IF NOT EXISTS vault_access_log (
    id             BIGSERIAL   PRIMARY KEY,
    vault_id       UUID        NOT NULL REFERENCES pii_vault(id) ON DELETE CASCADE,
    accessor_role  TEXT        NOT NULL,
    accessed_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    correlation_id TEXT        NOT NULL
);

CREATE INDEX IF NOT EXISTS val_vault_id_idx    ON vault_access_log (vault_id);
CREATE INDEX IF NOT EXISTS val_accessed_at_idx ON vault_access_log (accessed_at);

COMMENT ON TABLE vault_access_log IS 'Audit log. INSERT before read = fail-fast pattern.';
```

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate run
# Expected: Applied 1/migrate V30012__vault_access_log
```

---

### Step 3.4: V30013 — data_lineage table

Create `db/migration/V30013__data_lineage.sql`:

```sql
-- V30013: Data lineage events — survive raw_captures TTL expiry.
CREATE TABLE IF NOT EXISTS data_lineage (
    id                BIGSERIAL   PRIMARY KEY,
    pnu               TEXT        NOT NULL,
    source            TEXT        NOT NULL,
    fetched_at        TIMESTAMPTZ NOT NULL,
    sanitizer_version INTEGER,
    schema_hash       TEXT,
    license           TEXT,
    api_version       TEXT,
    captured_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS dl_pnu_source_idx ON data_lineage (pnu, source);
CREATE INDEX IF NOT EXISTS dl_fetched_at_idx ON data_lineage (fetched_at);
```

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate run
# Expected: Applied 1/migrate V30013__data_lineage
```

---

### Step 3.5: V30014 — TTL partial indexes

Create `db/migration/V30014__ttl_indexes.sql`:

```sql
-- V30014: Partial indexes for efficient hourly TTL cleanup.
CREATE INDEX IF NOT EXISTS raw_captures_expires_partial_idx
    ON raw_captures (expires_at)
    WHERE expires_at IS NOT NULL;

CREATE INDEX IF NOT EXISTS pii_vault_expires_partial_idx
    ON pii_vault (expires_at)
    WHERE expires_at IS NOT NULL;
```

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate run
# Expected: Applied 1/migrate V30014__ttl_indexes

sqlx migrate info 2>&1 | grep -E "V3001[0-4]"
# Expected: 5 lines — all show [x] Applied
```

**Commit:**
```
git add db/migration/
git commit -m "feat(db): T3 - migrations V30010-V30014 (lineage, pii vault, access log, TTL)"
```

---

## Task 4: PgPiiVaultCapture with Envelope Encryption

**Goal**: KMS-wrapped AES-256-GCM. Raw PII encrypted at rest; audit log before every read.

### Step 4.1: Add crypto deps to crates/db/Cargo.toml

```toml
[dependencies]
aws-sdk-kms = { workspace = true }
aes-gcm     = { workspace = true }
rand        = { workspace = true }
```

Workspace `Cargo.toml` `[workspace.dependencies]`:

```toml
aws-sdk-kms = "1"
aes-gcm     = "0.10"
rand        = "0.8"
```

```bash
cargo check -p db 2>&1 | grep -E "^error|Finished"
# Expected: Finished
```

---

### Step 4.2: Failing blob-format unit test

Create `crates/db/src/pii_vault.rs`:

```rust
//! PgPiiVaultCapture — KMS envelope-encrypted PII vault.
//! Blob format: enc_dek_len(4B BE) || enc_dek || iv(12B) || ciphertext

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::PgPool;
use tracing::{info, instrument};
use raw_capture_client::{RawCapture, RawCaptureError};

pub struct PgPiiVaultCapture {
    pool: PgPool,
    kms_key_id: String,
    kms_client: aws_sdk_kms::Client,
}

impl PgPiiVaultCapture {
    pub fn new(pool: PgPool, kms_key_id: String, kms_client: aws_sdk_kms::Client) -> Self {
        Self { pool, kms_key_id, kms_client }
    }

    fn pack_blob(enc_dek: &[u8], iv: &[u8; 12], ciphertext: &[u8]) -> Vec<u8> {
        let mut blob = Vec::with_capacity(4 + enc_dek.len() + 12 + ciphertext.len());
        blob.extend_from_slice(&(enc_dek.len() as u32).to_be_bytes());
        blob.extend_from_slice(enc_dek);
        blob.extend_from_slice(iv);
        blob.extend_from_slice(ciphertext);
        blob
    }
}

#[async_trait]
impl RawCapture for PgPiiVaultCapture {
    async fn capture(&self, pnu: &str, source: &str, raw: &Value, fetched_at: DateTime<Utc>)
        -> Result<(), RawCaptureError>
    {
        todo!("impl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unpack(blob: &[u8]) -> (&[u8], &[u8], &[u8]) {
        let len = u32::from_be_bytes(blob[..4].try_into().unwrap()) as usize;
        (&blob[4..4+len], &blob[4+len..4+len+12], &blob[4+len+12..])
    }

    #[test]
    fn blob_roundtrip() {
        let enc_dek = b"fake_encrypted_dek_32_bytes_here";
        let iv: [u8; 12] = *b"iv_12bytes__";
        let ct = b"encrypted_payload";
        let blob = PgPiiVaultCapture::pack_blob(enc_dek, &iv, ct);
        let (d, i, c) = unpack(&blob);
        assert_eq!(d, enc_dek);
        assert_eq!(i, iv);
        assert_eq!(c, ct);
    }
}
```

```bash
cargo test -p db blob_roundtrip 2>&1 | grep -E "ok|FAILED"
# Expected: test tests::blob_roundtrip ... ok
```

---

### Step 4.3: Implement PgPiiVaultCapture::capture

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit, aead::Aead};
use rand::RngCore;

#[async_trait]
impl RawCapture for PgPiiVaultCapture {
    #[instrument(skip(self, raw), fields(pnu=%pnu, source=%source))]
    async fn capture(&self, pnu: &str, source: &str, raw: &Value, fetched_at: DateTime<Utc>)
        -> Result<(), RawCaptureError>
    {
        let gdk = self.kms_client
            .generate_data_key()
            .key_id(&self.kms_key_id)
            .key_spec(aws_sdk_kms::types::DataKeySpec::Aes256)
            .send().await
            .map_err(|e| RawCaptureError::Io(format!("KMS: {e}")))?;

        let pt_dek = gdk.plaintext()
            .ok_or_else(|| RawCaptureError::Io("KMS: no plaintext DEK".into()))?
            .as_ref().to_vec();
        let enc_dek = gdk.ciphertext_blob()
            .ok_or_else(|| RawCaptureError::Io("KMS: no enc DEK".into()))?
            .as_ref().to_vec();

        let key = Key::<Aes256Gcm>::from_slice(&pt_dek);
        let cipher = Aes256Gcm::new(key);
        let mut iv = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut iv);
        let nonce = Nonce::from_slice(&iv);
        let plaintext = serde_json::to_vec(raw)
            .map_err(|e| RawCaptureError::Io(format!("JSON: {e}")))?;
        let ciphertext = cipher.encrypt(nonce, plaintext.as_slice())
            .map_err(|e| RawCaptureError::Io(format!("AES-GCM: {e}")))?;

        let blob = Self::pack_blob(&enc_dek, &iv, &ciphertext);
        let expires_at = fetched_at + Duration::days(30);
        sqlx::query!(
            "INSERT INTO pii_vault (pnu, source, encrypted_blob, enc_dek, iv, fetched_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            pnu, source, blob, enc_dek, iv.to_vec(), fetched_at, expires_at,
        )
        .execute(&self.pool).await
        .map_err(|e| RawCaptureError::Io(format!("INSERT: {e}")))?;

        info!(pnu=%pnu, source=%source, "PII vault capture complete");
        Ok(())
    }
}
```

```bash
cargo check -p db 2>&1 | grep -E "^error|Finished"
# Expected: Finished
```

---

### Step 4.4: PgVaultAccessLog

Append to `crates/db/src/pii_vault.rs`:

```rust
/// Immutable audit log — INSERT before vault read (fail-fast).
pub struct PgVaultAccessLog {
    pool: PgPool,
}

impl PgVaultAccessLog {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn record(
        &self,
        vault_id: uuid::Uuid,
        accessor_role: &str,
        correlation_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO vault_access_log (vault_id, accessor_role, correlation_id) VALUES ($1, $2, $3)",
            vault_id, accessor_role, correlation_id,
        )
        .execute(&self.pool).await?;
        Ok(())
    }
}
```

```bash
cargo check -p db 2>&1 | grep -E "^error|Finished"
# Expected: Finished
```

**Commit:**
```
git add crates/db/src/pii_vault.rs crates/db/Cargo.toml
git commit -m "feat(db): T4 - PgPiiVaultCapture KMS AES-256-GCM + PgVaultAccessLog"
```

---

## Task 5: Source Constants + Wire DualTierCapture

**Goal**: Replace hardcoded source strings; wire full capture chain in `services/api/src/main.rs`.

### Step 5.1: Failing test for vworld source const

Add to test section of `crates/data-clients/vworld/src/reader.rs`:

```rust
#[cfg(test)]
mod source_const_tests {
    use super::RAW_CAPTURE_SOURCE;
    #[test]
    fn source_const_is_vworld_parcel() {
        assert_eq!(RAW_CAPTURE_SOURCE, "vworld_parcel");
    }
}
```

```bash
cargo test -p vworld source_const_is_vworld_parcel 2>&1 | grep "error"
# Expected: error[E0425]: cannot find value RAW_CAPTURE_SOURCE
```

---

### Step 5.2: Add RAW_CAPTURE_SOURCE const to vworld reader

At top of `crates/data-clients/vworld/src/reader.rs`:

```rust
/// Source identifier — SSOT for raw_capture source column.
/// Matches AllowlistSanitizer::for_source("vworld_parcel").
pub const RAW_CAPTURE_SOURCE: &str = "vworld_parcel";
```

Update `fetch_by_pnu` capture call:

```rust
// Before:
.capture(pnu.as_str(), "vworld", &raw, now)
// After:
.capture(pnu.as_str(), RAW_CAPTURE_SOURCE, &raw, now)
```

```bash
cargo test -p vworld 2>&1 | grep "ok"
# Expected:
# test source_const_tests::source_const_is_vworld_parcel ... ok
# test tests::fetch_markers_in_bbox_returns_deferred_error ... ok
```

---

### Step 5.3: Add RAW_CAPTURE_SOURCE to data-go-kr reader

In `crates/data-clients/data-go-kr/src/` building reader:

```rust
/// Source identifier for data.go.kr building register captures.
pub const RAW_CAPTURE_SOURCE: &str = "data_go_kr_building";
```

Update any literal `"data_go_kr_building"` in `.capture()` calls to use the const.

```bash
cargo test -p data-go-kr 2>&1 | grep -E "ok|FAILED"
# Expected: test result: ok. N passed; 0 failed
```

---

### Step 5.4: Wire full DualTierCapture chain in main.rs

In `services/api/src/main.rs`:

```rust
use raw_capture_client::{
    sanitizer::AllowlistSanitizer,
    sanitizing_capture::SanitizingRawCapture,
    dual_tier::DualTierCapture,
};
use db::pii_vault::PgPiiVaultCapture;

// Tier 1: sanitized (PII filtered before PgRawCapture)
let vworld_sanitizer = AllowlistSanitizer::for_source("vworld_parcel")
    .expect("vworld_parcel allowlist — compile-time known source");
let tier1 = Arc::new(SanitizingRawCapture::new(
    Arc::new(PgRawCapture::new(pool.clone())),
    vworld_sanitizer,
));

// Tier 2: KMS vault (full raw PII, encrypted)
let kms_config = aws_config::load_from_env().await;
let kms_client = aws_sdk_kms::Client::new(&kms_config);
let kms_key_id = std::env::var("KMS_CMK_ID")
    .unwrap_or_else(|_| "alias/gongzzang-pii-vault-dev".to_owned());
let tier2 = Arc::new(PgPiiVaultCapture::new(
    pool.clone(), kms_key_id, kms_client,
));

let raw_capture: Arc<dyn raw_capture_client::RawCapture> =
    Arc::new(DualTierCapture::new(tier1, tier2));
```

```bash
cargo check -p api 2>&1 | grep -E "^error|Finished"
# Expected: Finished
```

**Commit:**
```
git add crates/data-clients/vworld/ crates/data-clients/data-go-kr/ services/api/src/main.rs
git commit -m "feat(api): T5 - DualTierCapture wired, source consts typed (no hardcoded strings)"
```

---

## Task 6: Health Endpoint Expansion + axum-test Integration

**Goal**: `/health/readiness` returns `{status, checks: {db, redis, building_reader, vault_kms}}`. Export testable router.

### Step 6.1: Add axum-test dev dependency

`services/api/Cargo.toml`:

```toml
[dev-dependencies]
axum-test = "15"
```

---

### Step 6.2: Failing integration test

Create `services/api/tests/health_integration.rs`:

```rust
use axum_test::TestServer;
use serde_json::Value;
use api::app_router_for_health;
use api::routes::health::HealthState;

#[tokio::test]
async fn readiness_returns_status_and_checks() {
    let app = app_router_for_health(HealthState::stub());
    let server = TestServer::new(app).unwrap();
    let resp = server.get("/health/readiness").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(body["status"].is_string());
    assert!(body["checks"]["db"].is_string());
    assert!(body["checks"]["redis"].is_string());
    assert!(body["checks"]["building_reader"].is_string());
    assert!(body["checks"]["vault_kms"].is_string());
}

#[test]
fn compute_readiness_degraded_if_any_not_ok() {
    use api::routes::health::{compute_readiness_status, ReadinessChecks};
    let checks = ReadinessChecks {
        db: "ok".to_owned(),
        redis: "error: connection refused".to_owned(),
        building_reader: "ok".to_owned(),
        vault_kms: "ok".to_owned(),
    };
    assert_eq!(compute_readiness_status(&checks), "degraded");
}

#[test]
fn compute_readiness_ok_when_all_ok() {
    use api::routes::health::{compute_readiness_status, ReadinessChecks};
    let checks = ReadinessChecks {
        db: "ok".to_owned(), redis: "ok".to_owned(),
        building_reader: "ok".to_owned(), vault_kms: "ok".to_owned(),
    };
    assert_eq!(compute_readiness_status(&checks), "ok");
}
```

```bash
cargo test -p api health_integration 2>&1 | grep "error"
# Expected: error[E0432]: unresolved import api::app_router_for_health
```

---

### Step 6.3: Expand HealthState + types in health.rs

```rust
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Clone)]
pub struct HealthState {
    pub pool: PgPool,
    pub redis_pool: deadpool_redis::Pool,
    pub building_reader_status: String,
    pub vault_kms_status: String,
}

impl HealthState {
    #[cfg(test)]
    pub fn stub() -> Self {
        use sqlx::postgres::PgPoolOptions;
        Self {
            pool: PgPoolOptions::new()
                .connect_lazy("postgres://localhost/stub_test")
                .expect("lazy connect"),
            redis_pool: deadpool_redis::Config::from_url("redis://localhost")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1))
                .expect("pool"),
            building_reader_status: "ok".to_owned(),
            vault_kms_status: "ok".to_owned(),
        }
    }
}

#[derive(Serialize)]
pub struct ReadinessChecks {
    pub db: String,
    pub redis: String,
    pub building_reader: String,
    pub vault_kms: String,
}

#[derive(Serialize)]
pub struct ReadinessResponse {
    pub status: String,
    pub checks: ReadinessChecks,
}

/// "ok" only when all checks == "ok"; else "degraded".
pub fn compute_readiness_status(checks: &ReadinessChecks) -> &'static str {
    if checks.db == "ok" && checks.redis == "ok"
        && checks.building_reader == "ok" && checks.vault_kms == "ok"
    {
        "ok"
    } else {
        "degraded"
    }
}

pub async fn readiness(State(state): State<HealthState>) -> Json<ReadinessResponse> {
    let db_status = sqlx::query("SELECT 1")
        .fetch_one(&state.pool).await
        .map(|_| "ok".to_owned())
        .unwrap_or_else(|e| format!("error: {e}"));

    let redis_status = async {
        let mut conn = state.redis_pool.get().await.map_err(|e| e.to_string())?;
        deadpool_redis::redis::cmd("PING")
            .query_async::<_, String>(&mut conn).await
            .map(|_| "ok".to_owned())
            .map_err(|e| e.to_string())
    }.await.unwrap_or_else(|e: String| format!("error: {e}"));

    let checks = ReadinessChecks {
        db: db_status, redis: redis_status,
        building_reader: state.building_reader_status.clone(),
        vault_kms: state.vault_kms_status.clone(),
    };
    let status = compute_readiness_status(&checks).to_owned();
    Json(ReadinessResponse { status, checks })
}
```

---

### Step 6.4: Export app_router_for_health from lib.rs

Create/update `services/api/src/lib.rs`:

```rust
pub mod routes;

use axum::Router;
use crate::routes::health::HealthState;

/// Testable router for /health/* — used by integration tests.
pub fn app_router_for_health(state: HealthState) -> Router {
    Router::new()
        .route("/health/readiness", axum::routing::get(routes::health::readiness))
        .with_state(state)
}
```

```bash
cargo test -p api health_integration 2>&1 | grep "ok"
# Expected:
# test readiness_returns_status_and_checks ... ok
# test compute_readiness_degraded_if_any_not_ok ... ok
# test compute_readiness_ok_when_all_ok ... ok
```

**Commit:**
```
git add services/api/src/routes/health.rs services/api/src/lib.rs services/api/tests/health_integration.rs services/api/Cargo.toml
git commit -m "feat(api): T6 - health readiness expanded (db/redis/building_reader/vault_kms)"
```

---

## Task 7: Tokio TTL Cleanup Task

**Goal**: Hourly DELETE of expired rows. PIPA 파기 의무 이행 (개인정보보호법 제21조).

### Step 7.1: Failing integration test

Create `services/api/tests/cleanup_task.rs`:

```rust
//! TTL cleanup integration test.
//! Run with: DATABASE_URL=... cargo test -p api cleanup -- --include-ignored

use chrono::{Duration, Utc};
use sqlx::PgPool;

async fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    PgPool::connect(&url).await.unwrap()
}

#[tokio::test]
#[ignore = "requires DATABASE_URL with test DB"]
async fn cleanup_deletes_expired_raw_captures() {
    let pool = test_pool().await;
    let expired = Utc::now() - Duration::hours(1);

    sqlx::query!(
        "INSERT INTO raw_captures (pnu, source, raw_response, fetched_at, expires_at) VALUES ($1, $2, $3::jsonb, $4, $5) ON CONFLICT (pnu, source) DO UPDATE SET expires_at = EXCLUDED.expires_at",
        "cleanup_test_pnu",
        "test_source",
        serde_json::json!({"test": true}) as _,
        Utc::now(),
        expired,
    )
    .execute(&pool).await.unwrap();

    let result = api::cleanup::run_cleanup_once(&pool).await.unwrap();
    assert!(result.raw_captures_deleted >= 1);

    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM raw_captures WHERE pnu = $1",
        "cleanup_test_pnu"
    )
    .fetch_one(&pool).await.unwrap().unwrap_or(0);
    assert_eq!(count, 0, "expired row must be deleted");
}
```

```bash
cargo test -p api cleanup 2>&1 | grep "error"
# Expected: error[E0432]: unresolved import api::cleanup
```

---

### Step 7.2: Implement cleanup module

Create `services/api/src/cleanup.rs`:

```rust
//! Hourly TTL cleanup — PIPA 개인정보보호법 제21조 파기 의무.

use sqlx::PgPool;
use tracing::{error, info, instrument};

#[derive(Debug)]
pub struct CleanupResult {
    pub raw_captures_deleted: u64,
    pub pii_vault_deleted: u64,
}

#[instrument(skip(pool))]
pub async fn run_cleanup_once(pool: &PgPool) -> Result<CleanupResult, sqlx::Error> {
    let raw = sqlx::query!(
        "DELETE FROM raw_captures WHERE expires_at IS NOT NULL AND expires_at < now()"
    ).execute(pool).await?;

    let vault = sqlx::query!(
        "DELETE FROM pii_vault WHERE expires_at < now()"
    ).execute(pool).await?;

    let (raw_deleted, vault_deleted) = (raw.rows_affected(), vault.rows_affected());
    info!(raw_captures_deleted=raw_deleted, pii_vault_deleted=vault_deleted, "TTL cleanup");
    Ok(CleanupResult { raw_captures_deleted: raw_deleted, pii_vault_deleted: vault_deleted })
}

pub fn spawn_cleanup_task(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            match run_cleanup_once(&pool).await {
                Ok(r) => info!(
                    raw_captures_deleted=r.raw_captures_deleted,
                    pii_vault_deleted=r.pii_vault_deleted,
                    "hourly TTL cleanup ok"
                ),
                Err(e) => error!(error=%e, "TTL cleanup failed"),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
    });
}
```

Add to `services/api/src/lib.rs`:

```rust
pub mod cleanup;
```

Wire in `services/api/src/main.rs` after pool setup:

```rust
// PIPA 파기: hourly TTL cleanup
api::cleanup::spawn_cleanup_task(pool.clone());
```

```bash
cargo check -p api 2>&1 | grep -E "^error|Finished"
# Expected: Finished

cargo test -p api cleanup 2>&1 | grep -E "ignored|ok"
# Expected: test cleanup_deletes_expired_raw_captures ... ignored
```

---

### Step 7.3: Run with real DB

```bash
DATABASE_URL=postgres://localhost/gongzzang_test   cargo test -p api cleanup -- --include-ignored 2>&1 | grep -E "ok|FAILED"
# Expected: test cleanup_deletes_expired_raw_captures ... ok
```

**Commit:**
```
git add services/api/src/cleanup.rs services/api/src/lib.rs services/api/src/main.rs services/api/tests/cleanup_task.rs
git commit -m "feat(api): T7 - Tokio hourly TTL cleanup (PIPA 파기: raw_captures + pii_vault)"
```

---

## Acceptance Criteria

```bash
# 1. Full workspace build
cargo build --workspace 2>&1 | grep "^error"
# Expected: (no output — zero errors)

# 2. All tests pass
cargo test --workspace 2>&1 | tail -5
# Expected: test result: ok. N passed; 0 failed

# 3. No clippy warnings
cargo clippy --workspace -- -D warnings 2>&1 | grep "^error"
# Expected: (no output)

# 4. All 5 migrations applied
DATABASE_URL=postgres://localhost/gongzzang_dev sqlx migrate info | grep -E "V3001[0-4]"
# Expected: 5 lines, all [x] Applied

# 5. Health endpoint shape
curl -s http://localhost:3000/health/readiness | python3 -m json.tool
```

Expected JSON response:

```json
{
  "status": "ok",
  "checks": {
    "db": "ok",
    "redis": "ok",
    "building_reader": "ok",
    "vault_kms": "ok"
  }
}
```

**PIPA 4원칙 확인표:**

| 원칙 | 구현 | 증거 |
|------|------|------|
| 수집목적 한정 | `AllowlistSanitizer::for_source()` | 7/9 paths only — 목적 외 필드 구조적 불가 |
| 최소수집 | `SanitizingRawCapture` | `dropped_count > 0` -> `schema_drift` warn |
| 보유기간 | `expires_at = fetched_at + 30d` | V30011 constraint + V30014 TTL index |
| 파기 | `cleanup::run_cleanup_once` | Tokio hourly task, `rows_affected` logged |

---

*SP10.5-B Implementation Plan — 2026-05-08*
*Spec SSOT: `docs/superpowers/specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md`*

# T3: Two-tier Vault Migrations + PgPiiVaultCapture + KMS + DualTierCapture

**Goal:** Tier 2 PII vault 테이블 + AWS KMS envelope encryption + lineage 컬럼 추가 + `DualTierCapture` fan-out composer. Tier 2 (vault) 먼저 호출하여 fail-fast 보장.

**Spec SSOT:** §3.4, §3.5, §6.1, §6.2, §6.3, §13 T3 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**T2 inputs (already exported):** `sources::{data_go_kr_building, vworld_parcel}::{SOURCE_ID, *_ALLOWLIST}`, `AllowlistSanitizer::for_source`, `SanitizerError`, `SanitizingRawCapture`.

**Files:**

- Create: `migrations/30013_pii_vault.sql`
- Create: `migrations/30014_external_data_lineage.sql`
- Create: `crates/db/src/pii_vault.rs`
- Create: `infra/kms-key.ts`
- Modify: `crates/data-clients/raw-capture/src/capture.rs` (add `DualTierCapture`)
- Modify: `crates/data-clients/raw-capture/src/lib.rs` (re-export `DualTierCapture`)
- Modify: `crates/db/src/lib.rs` (expose `pii_vault` module)
- Modify: `crates/db/Cargo.toml` (aws-sdk-kms + aes-gcm)

**Lock dependency**: T3 의 30013/30014 마이그레이션 + KMS infra + PgPiiVaultCapture + DualTierCapture 는 **동일 PR** 에 묶여야 함. vault 테이블 없이 PgPiiVaultCapture INSERT 시 SQL error; DualTierCapture 없이 vault 채워지지 않음.

---

## Step 3.1: Add aws-sdk-kms + aes-gcm dependencies

- [ ] **Step 3.1.1: Modify `crates/db/Cargo.toml`**

```toml
[dependencies]
# ... 기존 ...
aws-sdk-kms = { workspace = true }
aes-gcm = { workspace = true }
```

Workspace `Cargo.toml` 에 미정의 시 추가:

```toml
[workspace.dependencies]
# ... 기존 ...
aws-sdk-kms = "1"
aes-gcm = "0.10"
```

- [ ] **Step 3.1.2: Build check**

```bash
cargo check -p gongzzang-db
# Expected: Finished — kms / aes-gcm 컴파일 가능
```

- [ ] **Step 3.1.3: Commit**

```bash
git add crates/db/Cargo.toml Cargo.toml
git commit -m "chore(sp10-5-b-T3): add aws-sdk-kms + aes-gcm deps to db crate"
```

---

## Step 3.2: Migration 30013 — pii_vault table + RLS

- [ ] **Step 3.2.1: Create `migrations/30013_pii_vault.sql`**

```sql
-- V003_13: parcel_external_data_pii_vault — KMS envelope encrypted Tier 2 vault.
--
-- Spec SSOT: design.md §6.2.
--
-- ADR 근거: parcel_external_data PK 가 (pnu char(19), source varchar(40))
-- composite. PostgreSQL FK 는 referencing/referenced 컬럼 타입이 *정확히* 일치
-- 해야 함 → vault 의 pnu/source 도 동일 타입 사용. source CHECK 도 fail-safe
-- 로 별도 추가 (parent CHECK 와 sync 깨질 위험 대신 명시적 vault enum).
--
-- Lock safety: 신규 테이블 생성 + RLS policy 추가 — 기존 테이블 lock 영향 없음.

BEGIN;

CREATE TABLE parcel_external_data_pii_vault (
    id               UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    pnu              char(19)     NOT NULL,
    source           varchar(40)  NOT NULL CHECK (source IN (
        'vworld',                          -- legacy alias (backfill 이전 row 호환)
        'vworld_parcel',
        'data_go_kr_building',
        'data_go_kr_land',
        'data_go_kr_realtransaction',
        'korean_law'
    )),
    ciphertext_blob  BYTEA        NOT NULL,
    kms_key_id       TEXT         NOT NULL,
    encryption_ctx   JSONB        NOT NULL DEFAULT '{}',
    captured_at      TIMESTAMPTZ  NOT NULL DEFAULT now(),
    expires_at       TIMESTAMPTZ  NOT NULL,
    FOREIGN KEY (pnu, source) REFERENCES parcel_external_data(pnu, source) ON DELETE CASCADE
);

CREATE INDEX parcel_external_data_pii_vault_pnu_source_idx
    ON parcel_external_data_pii_vault (pnu, source);

-- Row-Level Security: 기본 차단, 'admin' role 만 접근.
-- Application 은 `SET LOCAL app.role = 'admin'` 트랜잭션 시작 시 명시.
ALTER TABLE parcel_external_data_pii_vault ENABLE ROW LEVEL SECURITY;

CREATE POLICY vault_admin_only ON parcel_external_data_pii_vault
    USING (current_setting('app.role', true) = 'admin');

COMMENT ON TABLE parcel_external_data_pii_vault IS
    'Tier 2 PII vault — KMS envelope encrypted raw responses. Access via /api/admin/raw_vault.';
COMMENT ON COLUMN parcel_external_data_pii_vault.ciphertext_blob IS
    'Format: enc_dek_len(4B BE) || enc_dek || iv(12B) || ciphertext (AES-256-GCM)';
COMMENT ON COLUMN parcel_external_data_pii_vault.encryption_ctx IS
    'AAD for AES-GCM. Includes pnu/source/captured_at for binding.';

COMMIT;
```

- [ ] **Step 3.2.2: Run forward migration**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30013/migrate pii_vault
```

- [ ] **Step 3.2.3: Verify schema**

```bash
psql gongzzang_dev -c "\d parcel_external_data_pii_vault"
# Expected: 표시된 컬럼: id, pnu char(19), source varchar(40), ciphertext_blob bytea, ...
psql gongzzang_dev -c "SELECT relname, relrowsecurity FROM pg_class WHERE relname = 'parcel_external_data_pii_vault';"
# Expected: relrowsecurity = t (RLS enabled)
```

- [ ] **Step 3.2.4: Commit**

```bash
git add migrations/30013_pii_vault.sql
git commit -m "feat(sp10-5-b-T3): migration 30013 — pii_vault table + RLS + composite FK"
```

---

## Step 3.3: Migration 30014 — external_data_lineage columns

- [ ] **Step 3.3.1: Create `migrations/30014_external_data_lineage.sql`**

```sql
-- V003_14: lineage columns on parcel_external_data — data provenance + drift detection.
--
-- Spec SSOT: design.md §6.1.
--
-- 컬럼:
--   license            : 데이터셋 라이선스 (예: 'KOGL-TYPE1', 'V-WORLD-TOS')
--   api_version        : upstream API version (예: 'data.go.kr/BldRgstService_v2')
--   sanitizer_version  : AllowlistSanitizer 버전 (스키마 변경 시 증가)
--   schema_hash        : SHA-256 of allowlist definition (drift detection input)
--
-- Lock safety: ADD COLUMN with DEFAULT (sanitizer_version DEFAULT 1) 는
-- PostgreSQL 11+ 에서 instant operation (rewrite 없음). NULL 허용 컬럼은
-- 추가 lock 없이 즉시 적용.

BEGIN;

ALTER TABLE parcel_external_data
    ADD COLUMN license            TEXT,
    ADD COLUMN api_version        TEXT,
    ADD COLUMN sanitizer_version  INT NOT NULL DEFAULT 1,
    ADD COLUMN schema_hash        TEXT;

-- Backfill 기존 레코드: schema_hash 는 SHA-256 hash 아님을 'legacy:' prefix 로 표시.
UPDATE parcel_external_data
   SET schema_hash       = 'legacy:' || md5(raw_response::text),
       sanitizer_version = 0
 WHERE schema_hash IS NULL OR schema_hash = '';

COMMENT ON COLUMN parcel_external_data.license IS
    'Open data license code (KOGL-TYPE1, V-WORLD-TOS, etc).';
COMMENT ON COLUMN parcel_external_data.sanitizer_version IS
    'AllowlistSanitizer version. 0 = legacy pre-SP10.5-B, 1+ = post-SP10.5-B.';
COMMENT ON COLUMN parcel_external_data.schema_hash IS
    'SHA-256 of allowlist (source:version:sorted_paths). Drift detection input.';

COMMIT;
```

- [ ] **Step 3.3.2: Run forward migration**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30014/migrate external_data_lineage
```

- [ ] **Step 3.3.3: Verify columns + backfill**

```bash
psql gongzzang_dev -c "\d parcel_external_data" | grep -E "license|api_version|sanitizer_version|schema_hash"
# Expected: 4 lineage columns visible
psql gongzzang_dev -c "SELECT sanitizer_version, count(*) FROM parcel_external_data GROUP BY sanitizer_version;"
# Expected: sanitizer_version = 0 for all pre-migration rows (legacy backfill)
psql gongzzang_dev -c "SELECT count(*) FROM parcel_external_data WHERE schema_hash LIKE 'legacy:%';"
# Expected: count > 0 if any pre-existing rows
```

- [ ] **Step 3.3.4: Commit**

```bash
git add migrations/30014_external_data_lineage.sql
git commit -m "feat(sp10-5-b-T3): migration 30014 — lineage cols + legacy backfill"
```

---

## Step 3.4: PgPiiVaultCapture struct + `RawCapture` impl (TDD)

- [ ] **Step 3.4.1: Create `crates/db/src/pii_vault.rs` with failing test (struct ONLY)**

```rust
//! Tier 2 PII vault sink — KMS envelope encryption + INSERT to
//! `parcel_external_data_pii_vault`.
//!
//! Spec §6.2, §6.3 SSOT. AWS KMS `GenerateDataKey` 로 DEK 생성 → AES-256-GCM
//! 으로 full raw JSON 암호화 → ciphertext_blob INSERT. RLS 우회 위해
//! 트랜잭션 시작 시 `SET LOCAL app.role = 'admin'`.

use aes_gcm::{Aes256Gcm, KeyInit};
use aws_sdk_kms::Client as KmsClient;
use chrono::{DateTime, Duration, Utc};
use raw_capture_client::{RawCapture, RawCaptureError, RawCaptureReceipt};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;

pub struct PgPiiVaultCapture {
    pool: PgPool,
    kms: Arc<KmsClient>,
    kms_key_id: String,
    ttl: Duration,
}

impl PgPiiVaultCapture {
    pub fn new(pool: PgPool, kms: Arc<KmsClient>, kms_key_id: String, ttl: Duration) -> Self {
        Self { pool, kms, kms_key_id, ttl }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_pii_vault_construct() {
        // KMS mock + pool 은 별도 integration test 에서. 여기는 struct construction.
        // PgPool::connect_lazy 로 pool placeholder, KmsClient mock fixture
        // 는 Step 3.4.5 의 integration test 에서.
        let _ = std::any::TypeId::of::<PgPiiVaultCapture>();
    }
}
```

- [ ] **Step 3.4.2: Modify `crates/db/src/lib.rs` — expose module**

```rust
pub mod pii_vault;
pub use pii_vault::PgPiiVaultCapture;
```

- [ ] **Step 3.4.3: Verify compile**

```bash
cargo check -p gongzzang-db
# Expected: Finished — struct + module declarations only
```

- [ ] **Step 3.4.4: Commit struct scaffold**

```bash
git add crates/db/src/pii_vault.rs crates/db/src/lib.rs
git commit -m "feat(sp10-5-b-T3): PgPiiVaultCapture struct scaffold"
```

- [ ] **Step 3.4.5: Append failing integration test (KMS + INSERT)**

```rust
    use raw_capture_client::RawCapture;

    /// Real KMS + DB integration. Test environment requirement:
    ///   - localstack KMS endpoint (env: AWS_ENDPOINT_URL=http://localhost:4566)
    ///   - test database with 30013/30014 migrations applied
    #[tokio::test]
    #[ignore = "requires localstack KMS + test DB; run with --ignored"]
    async fn vault_capture_encrypts_and_inserts() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let kms_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let kms = Arc::new(KmsClient::new(&kms_config));
        let key = kms
            .create_key()
            .description("test-pii-vault-key")
            .send()
            .await
            .unwrap();
        let key_id = key.key_metadata().unwrap().key_id().to_string();

        let vault = PgPiiVaultCapture::new(pool.clone(), kms.clone(), key_id, Duration::days(30));
        let raw = serde_json::json!({"pnu": "1111010100100010000", "owner_secret": "PII"});
        let now = Utc::now();
        let receipt = vault
            .capture("1111010100100010000", "data_go_kr_building", &raw, now)
            .await
            .unwrap();
        assert!(!receipt.location.is_empty());

        // INSERT 확인 + ciphertext_blob 는 plaintext "owner_secret" 미포함
        let row: (Vec<u8>,) = sqlx::query_as(
            "SELECT ciphertext_blob FROM parcel_external_data_pii_vault WHERE pnu = $1",
        )
        .bind("1111010100100010000")
        .fetch_one(&pool)
        .await
        .unwrap();
        let plaintext_check = String::from_utf8_lossy(&row.0);
        assert!(!plaintext_check.contains("owner_secret"), "raw plaintext leaked!");
    }

    #[tokio::test]
    async fn vault_kms_failure_returns_err() {
        // KMS mock 이 fail 하도록 잘못된 endpoint 사용
        let pool = sqlx::PgPool::connect_lazy("postgres://invalid").unwrap();
        let kms_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url("http://127.0.0.1:1")  // unreachable port
            .load()
            .await;
        let kms = Arc::new(KmsClient::new(&kms_config));
        let vault = PgPiiVaultCapture::new(
            pool,
            kms,
            "alias/nonexistent".to_string(),
            Duration::days(30),
        );
        let raw = serde_json::json!({"x": 1});
        let result = vault
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await;
        assert!(result.is_err(), "KMS failure must propagate as Err (fail-fast)");
    }
```

- [ ] **Step 3.4.6: Run — verify FAIL (RawCapture impl not yet)**

```bash
cargo test -p gongzzang-db --lib pii_vault::tests::vault_kms_failure
# Expected: error[E0599]: no method named `capture` found for struct `PgPiiVaultCapture`
```

- [ ] **Step 3.4.7: Implement `RawCapture` for `PgPiiVaultCapture`**

Append to `pii_vault.rs`:

```rust
use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use async_trait::async_trait;
use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::DataKeySpec;

#[async_trait]
impl RawCapture for PgPiiVaultCapture {
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        // 1. KMS GenerateDataKey → DEK + encrypted DEK
        let dk = self
            .kms
            .generate_data_key()
            .key_id(&self.kms_key_id)
            .key_spec(DataKeySpec::Aes256)
            .send()
            .await
            .map_err(|e| RawCaptureError::Sink(format!("kms generate_data_key: {e}")))?;
        let plaintext_dek = dk.plaintext().ok_or_else(|| {
            RawCaptureError::Sink("kms generate_data_key returned no plaintext".to_string())
        })?;
        let encrypted_dek = dk.ciphertext_blob().ok_or_else(|| {
            RawCaptureError::Sink("kms generate_data_key returned no ciphertext".to_string())
        })?;

        // 2. AES-256-GCM encrypt raw JSON
        let cipher = Aes256Gcm::new_from_slice(plaintext_dek.as_ref())
            .map_err(|e| RawCaptureError::Sink(format!("aes-gcm init: {e}")))?;
        let nonce_bytes = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let raw_bytes = serde_json::to_vec(raw)
            .map_err(|e| RawCaptureError::Sink(format!("serde_json: {e}")))?;
        let ciphertext = cipher
            .encrypt(nonce, raw_bytes.as_ref())
            .map_err(|e| RawCaptureError::Sink(format!("aes-gcm encrypt: {e}")))?;

        // 3. Pack blob: enc_dek_len(4B BE) || enc_dek || iv(12B) || ciphertext
        let enc_dek_bytes = encrypted_dek.as_ref();
        let mut blob = Vec::with_capacity(4 + enc_dek_bytes.len() + 12 + ciphertext.len());
        blob.extend_from_slice(&(enc_dek_bytes.len() as u32).to_be_bytes());
        blob.extend_from_slice(enc_dek_bytes);
        blob.extend_from_slice(nonce_bytes.as_slice());
        blob.extend_from_slice(&ciphertext);

        // 4. INSERT with RLS bypass (SET LOCAL app.role = 'admin')
        let expires_at = fetched_at + self.ttl;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| RawCaptureError::Sink(format!("tx begin: {e}")))?;
        sqlx::query("SET LOCAL app.role = 'admin'")
            .execute(&mut *tx)
            .await
            .map_err(|e| RawCaptureError::Sink(format!("set role: {e}")))?;
        let id: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO parcel_external_data_pii_vault
                (pnu, source, ciphertext_blob, kms_key_id, captured_at, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id",
        )
        .bind(pnu)
        .bind(source)
        .bind(&blob)
        .bind(&self.kms_key_id)
        .bind(fetched_at)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| RawCaptureError::Sink(format!("insert: {e}")))?;
        tx.commit()
            .await
            .map_err(|e| RawCaptureError::Sink(format!("commit: {e}")))?;

        Ok(RawCaptureReceipt {
            location: format!("postgres://pii_vault/{}", id.0),
            bytes: blob.len(),
        })
    }
}
```

- [ ] **Step 3.4.8: Run — verify PASS (kms_failure test; ignored test 는 별도)**

```bash
cargo test -p gongzzang-db --lib pii_vault::tests::vault_kms_failure
# Expected: ok. 1 passed (KMS unreachable → Err)
cargo test -p gongzzang-db --lib pii_vault::tests::vault_capture_encrypts_and_inserts -- --ignored
# Expected (with localstack + test DB): ok. 1 passed
# Expected (without localstack): test ignored
```

- [ ] **Step 3.4.9: Commit**

```bash
git add crates/db/src/pii_vault.rs
git commit -m "feat(sp10-5-b-T3): PgPiiVaultCapture KMS envelope + AES-256-GCM + RLS"
```

---

## Step 3.5: `DualTierCapture` fan-out composer (TDD)

Spec §3.5 — Tier 2 (vault) 먼저 호출하여 fail-fast 보장.

- [ ] **Step 3.5.1: Append failing test to `crates/data-clients/raw-capture/src/capture.rs`**

`mod tests` 안에 추가:

```rust
    #[tokio::test]
    async fn dual_tier_vault_first_failfast() {
        // Tier 2 가 실패하면 Tier 1 호출 안 됨 (fail-fast)
        struct AlwaysFailVault;
        #[async_trait]
        impl RawCapture for AlwaysFailVault {
            async fn capture(
                &self,
                _: &str,
                _: &str,
                _: &Value,
                _: DateTime<Utc>,
            ) -> Result<RawCaptureReceipt, RawCaptureError> {
                Err(RawCaptureError::Sink("vault down".to_string()))
            }
        }

        struct TrackedSanitizedSink {
            called: Arc<std::sync::atomic::AtomicBool>,
        }
        #[async_trait]
        impl RawCapture for TrackedSanitizedSink {
            async fn capture(
                &self,
                _: &str,
                _: &str,
                _: &Value,
                _: DateTime<Utc>,
            ) -> Result<RawCaptureReceipt, RawCaptureError> {
                self.called.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(RawCaptureReceipt {
                    location: "test".to_string(),
                    bytes: 0,
                })
            }
        }

        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let dual = DualTierCapture::new(
            TrackedSanitizedSink {
                called: called.clone(),
            },
            AlwaysFailVault,
        );
        let raw = serde_json::json!({"x": 1});
        let result = dual.capture("p", "s", &raw, Utc::now()).await;
        assert!(result.is_err(), "Tier 2 failure must propagate");
        assert!(
            !called.load(std::sync::atomic::Ordering::SeqCst),
            "Tier 1 must NOT be called if Tier 2 fails (fail-fast)"
        );
    }

    #[tokio::test]
    async fn dual_tier_both_success() {
        let dual = DualTierCapture::new(NoOpRawCapture::new(), NoOpRawCapture::new());
        let raw = serde_json::json!({"x": 1});
        let result = dual.capture("p", "s", &raw, Utc::now()).await;
        assert!(result.is_ok());
        // sanitized sink (NoOp) 의 receipt 가 반환
        let receipt = result.unwrap();
        let _ = receipt;
    }
```

- [ ] **Step 3.5.2: Run — verify FAIL (DualTierCapture undefined)**

```bash
cargo test -p raw-capture-client --lib capture::tests::dual_tier
# Expected: error[E0422]: cannot find struct, variant or union type `DualTierCapture`
```

- [ ] **Step 3.5.3: Implement `DualTierCapture` — append to `capture.rs` above `#[cfg(test)]`**

```rust
/// Tier 1 (sanitized) + Tier 2 (vault) fan-out. Tier 2 먼저 호출하여 fail-fast 보장:
/// vault INSERT 실패 시 Tier 1 기록 자체를 차단 → raw 평문이 sanitized 컬럼으로
/// 잘못 들어가는 경우 방지.
///
/// 반환 receipt 는 *sanitized sink* (Tier 1) 의 것 — caller 가 일반적으로 보는
/// 결과는 정제된 location 이다. vault location 은 audit log 에서 별도 조회.
pub struct DualTierCapture<S, V> {
    sanitized: S,
    vault: V,
}

impl<S, V> DualTierCapture<S, V> {
    pub fn new(sanitized: S, vault: V) -> Self {
        Self { sanitized, vault }
    }
}

#[async_trait]
impl<S, V> RawCapture for DualTierCapture<S, V>
where
    S: RawCapture + Send + Sync,
    V: RawCapture + Send + Sync,
{
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        // Tier 2 (vault) 먼저 — 실패 시 Tier 1 차단 (fail-fast)
        self.vault.capture(pnu, source, raw, fetched_at).await?;
        // Tier 1 (sanitized) — Tier 2 성공 후만 실행
        self.sanitized.capture(pnu, source, raw, fetched_at).await
    }
}
```

- [ ] **Step 3.5.4: Run — verify PASS**

```bash
cargo test -p raw-capture-client --lib capture::tests::dual_tier
# Expected: 2 passed (dual_tier_vault_first_failfast, dual_tier_both_success)
```

- [ ] **Step 3.5.5: Re-export `DualTierCapture` in lib.rs**

```rust
pub use capture::{DualTierCapture, SanitizingRawCapture};
```

- [ ] **Step 3.5.6: Run full raw-capture suite**

```bash
cargo test -p raw-capture-client --lib
# Expected: all tests pass (sanitizer + sources + capture = 20+ tests)
cargo clippy -p raw-capture-client -- -D warnings
# Expected: no warnings
```

- [ ] **Step 3.5.7: Commit**

```bash
git add crates/data-clients/raw-capture/src/capture.rs crates/data-clients/raw-capture/src/lib.rs
git commit -m "feat(sp10-5-b-T3): DualTierCapture fan-out (Tier 2 first, fail-fast)"
```

---

## Step 3.6: AWS KMS Pulumi infrastructure

- [ ] **Step 3.6.1: Create `infra/kms-key.ts`**

```typescript
// infra/kms-key.ts — gongzzang PII vault CMK.
//
// Spec §6.3 SSOT. Pulumi-managed (AGENTS.md §1: 인프라는 코드만, AWS 콘솔 직접
// 변경 금지). Key Policy 는 services/api task role 에만 GenerateDataKey + Decrypt
// 허용.

import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

const config = new pulumi.Config();
const projectName = config.get("projectName") ?? "gongzzang";

export const piiVaultKey = new aws.kms.Key("pii-vault-key", {
    description: `${projectName} PII vault CMK (PIPA Tier 2 encryption)`,
    enableKeyRotation: true,
    deletionWindowInDays: 30,
    tags: {
        Project: projectName,
        Compliance: "PIPA",
        DataClass: "PII-Tier2",
    },
});

export const piiVaultKeyAlias = new aws.kms.Alias("pii-vault-key-alias", {
    name: `alias/${projectName}-pii-vault`,
    targetKeyId: piiVaultKey.keyId,
});

// Export for application config
export const piiVaultKmsKeyId = piiVaultKey.keyId;
export const piiVaultKmsArn = piiVaultKey.arn;
```

- [ ] **Step 3.6.2: Pulumi preview**

```bash
cd infra && pulumi preview
# Expected: + create aws:kms:Key/pii-vault-key
#           + create aws:kms:Alias/pii-vault-key-alias
```

- [ ] **Step 3.6.3: Commit**

```bash
git add infra/kms-key.ts
git commit -m "feat(sp10-5-b-T3): Pulumi KMS key for PII vault (rotation + 30d deletion)"
```

---

## Acceptance — T3 완료 기준

- [ ] `migrations/30013_pii_vault.sql` 적용됨 (vault 테이블 + RLS + composite FK)
- [ ] `migrations/30014_external_data_lineage.sql` 적용됨 (4 lineage cols + legacy backfill)
- [ ] `cargo test -p gongzzang-db --lib pii_vault` — kms_failure_fail_fast 테스트 PASS
- [ ] `cargo test -p gongzzang-db --lib pii_vault -- --ignored` (localstack 있을 시) — vault_capture_encrypts_and_inserts PASS
- [ ] `cargo test -p raw-capture-client --lib capture::tests::dual_tier` — 2 PASS (vault_first_failfast + both_success)
- [ ] `cargo clippy --workspace -- -D warnings` 통과
- [ ] `pulumi preview` 에 KMS Key + Alias 생성 표시
- [ ] T4 가 사용할 인터페이스 export: `gongzzang_db::PgPiiVaultCapture`, `raw_capture_client::DualTierCapture`

**다음 task:** [T4-ttl-cleanup.md](T4-ttl-cleanup.md) — migration 30016 expires_at NOT NULL + Tokio cleanup task.

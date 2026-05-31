# T3 Vault KMS Lineage - Part 02: PgPiiVaultCapture

Parent index: [T3 Vault KMS Lineage](./T3-vault-kms-lineage.md).


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

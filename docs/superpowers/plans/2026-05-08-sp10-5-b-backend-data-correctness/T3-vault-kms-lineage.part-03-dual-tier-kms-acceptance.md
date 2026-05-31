# T3 Vault KMS Lineage - Part 03: DualTierCapture, KMS Infra, And Acceptance

Parent index: [T3 Vault KMS Lineage](./T3-vault-kms-lineage.md).


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

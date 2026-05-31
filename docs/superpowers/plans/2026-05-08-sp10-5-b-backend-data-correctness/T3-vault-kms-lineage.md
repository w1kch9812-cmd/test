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

## Plan Parts

Detailed step bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Dependencies, Vault Migration, And Lineage](./T3-vault-kms-lineage.part-01-deps-migration-lineage.md)
- [Part 02 - PgPiiVaultCapture](./T3-vault-kms-lineage.part-02-pg-pii-vault-capture.md)
- [Part 03 - DualTierCapture, KMS Infra, And Acceptance](./T3-vault-kms-lineage.part-03-dual-tier-kms-acceptance.md)

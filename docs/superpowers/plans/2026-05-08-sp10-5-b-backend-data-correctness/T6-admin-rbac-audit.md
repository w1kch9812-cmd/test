# T6: Vault Admin Endpoint + Audit Log + ZITADEL RBAC

**Goal:** `GET /api/admin/raw_vault/:source/:pnu` 신규 endpoint. ZITADEL admin role + `purpose` enum + `ticket_id` 필수. 모든 호출이 `raw_vault_access_log` INSERT (fail-fast). KMS decrypt 후 full raw JSON 반환.

**Spec SSOT:** §6.4 (Admin Endpoint Contract), §13 T6 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**T3 inputs:** `PgPiiVaultCapture` (vault row 존재). T5 inputs: KMS client + pool.

**Files:**

- Create: `migrations/30015_raw_vault_access_log.sql`
- Create: `crates/db/src/access_log.rs`
- Create: `services/api/src/routes/admin/mod.rs`
- Create: `services/api/src/routes/admin/raw_vault.rs`
- Modify: `crates/db/src/lib.rs` (expose `access_log`)
- Modify: `services/api/src/main.rs` (mount admin router)

---

## Plan Parts

Detailed step bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Migration And Access Log Repository](./T6-admin-rbac-audit.part-01-migration-access-log.md)
- [Part 02 - Admin Endpoint Handler](./T6-admin-rbac-audit.part-02-admin-endpoint.md)
- [Part 03 - Router Mount And Integration Gate](./T6-admin-rbac-audit.part-03-router-integration-gate.md)

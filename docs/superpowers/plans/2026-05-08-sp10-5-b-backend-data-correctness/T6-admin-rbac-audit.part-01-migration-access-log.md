# T6 Vault Admin RBAC Audit - Part 01: Migration And Access Log Repository

Parent index: [T6 Vault Admin RBAC Audit](./T6-admin-rbac-audit.md).

## Step 6.1: Migration 30015 — raw_vault_access_log table

- [ ] **Step 6.1.1: Create `migrations/30015_raw_vault_access_log.sql`**

```sql
-- V003_15: raw_vault_access_log — admin 조회 audit log.
--
-- Spec SSOT: design.md §6.4. PIPA 추적성 — 누가 언제 raw 조회 했는지 영구 기록.
--
-- 컬럼 (spec §6.4 + §11 SSOT, 7개):
--   user_id     : ZITADEL sub claim
--   source      : vault row 의 source (FK ref X — audit 는 vault 삭제 후에도 보존)
--   pnu         : vault row 의 pnu
--   purpose     : enum (incident_investigation / drift_diagnosis / customer_request)
--   ticket_id   : 외부 ticketing 시스템 correlation
--   accessed_at : timestamp (DEFAULT now)
--   request_id  : end-to-end trace correlation (X-Request-Id 헤더)

BEGIN;

CREATE TABLE raw_vault_access_log (
    id              BIGSERIAL    PRIMARY KEY,
    user_id         TEXT         NOT NULL,
    source          varchar(40)  NOT NULL,
    pnu             char(19)     NOT NULL,
    purpose         TEXT         NOT NULL CHECK (purpose IN (
        'incident_investigation',
        'drift_diagnosis',
        'customer_request'
    )),
    ticket_id       TEXT         NOT NULL,
    accessed_at     TIMESTAMPTZ  NOT NULL DEFAULT now(),
    request_id      TEXT         NOT NULL
);

CREATE INDEX raw_vault_access_log_pnu_source_idx
    ON raw_vault_access_log (pnu, source);

CREATE INDEX raw_vault_access_log_accessed_at_idx
    ON raw_vault_access_log (accessed_at);

CREATE INDEX raw_vault_access_log_user_id_idx
    ON raw_vault_access_log (user_id);

COMMENT ON TABLE raw_vault_access_log IS
    'PIPA audit log — every vault access. Immutable (INSERT only).';
COMMENT ON COLUMN raw_vault_access_log.purpose IS
    'PIPA 수집 목적. application 이 enum 강제, DB 가 CHECK 으로 fail-safe.';

COMMIT;
```

- [ ] **Step 6.1.2: Run forward migration + verify**

```bash
DATABASE_URL=postgres://localhost/gongzzang_dev cargo sqlx migrate run
# Expected: Applied 30015/migrate raw_vault_access_log
psql gongzzang_dev -c "\d raw_vault_access_log"
# Expected: 7 columns + 3 indexes
```

- [ ] **Step 6.1.3: Commit**

```bash
git add migrations/30015_raw_vault_access_log.sql
git commit -m "feat(sp10-5-b-T6): migration 30015 — raw_vault_access_log (7 columns)"
```

---

## Step 6.2: PgVaultAccessLog struct + record method (TDD)

- [ ] **Step 6.2.1: Create `crates/db/src/access_log.rs` with failing test**

```rust
//! `raw_vault_access_log` INSERT — PIPA audit. fail-fast on insert failure.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct AccessLogEntry {
    pub user_id: String,
    pub source: String,
    pub pnu: String,
    pub purpose: AccessPurpose,
    pub ticket_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPurpose {
    IncidentInvestigation,
    DriftDiagnosis,
    CustomerRequest,
}

impl AccessPurpose {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Self::IncidentInvestigation => "incident_investigation",
            Self::DriftDiagnosis => "drift_diagnosis",
            Self::CustomerRequest => "customer_request",
        }
    }
}

pub struct PgVaultAccessLog {
    pool: PgPool,
}

impl PgVaultAccessLog {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// audit 행 INSERT. 실패는 caller 가 *fail-fast* — vault SELECT 전에 호출되어야
    /// 함 (응답 전 audit 보장).
    pub async fn record(
        &self,
        entry: AccessLogEntry,
        accessed_at: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        unimplemented!("Step 6.2.4 에서 impl")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn purpose_db_str() {
        assert_eq!(AccessPurpose::IncidentInvestigation.as_db_str(), "incident_investigation");
        assert_eq!(AccessPurpose::DriftDiagnosis.as_db_str(), "drift_diagnosis");
        assert_eq!(AccessPurpose::CustomerRequest.as_db_str(), "customer_request");
    }

    #[tokio::test]
    #[ignore = "requires test DB with 30015 migration"]
    async fn record_inserts_row() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let log = PgVaultAccessLog::new(pool.clone());
        let id = log
            .record(
                AccessLogEntry {
                    user_id: "user-1".to_string(),
                    source: "data_go_kr_building".to_string(),
                    pnu: "1111010100100010003".to_string(),
                    purpose: AccessPurpose::DriftDiagnosis,
                    ticket_id: "TICKET-123".to_string(),
                    request_id: "req-abc".to_string(),
                },
                Utc::now(),
            )
            .await
            .unwrap();
        assert!(id > 0);

        let row: (String, String) = sqlx::query_as(
            "SELECT purpose, ticket_id FROM raw_vault_access_log WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, "drift_diagnosis");
        assert_eq!(row.1, "TICKET-123");
    }
}
```

- [ ] **Step 6.2.2: Modify `crates/db/src/lib.rs` — expose**

```rust
pub mod access_log;
pub use access_log::{AccessLogEntry, AccessPurpose, PgVaultAccessLog};
```

- [ ] **Step 6.2.3: Run — verify FAIL (unimplemented)**

```bash
cargo test -p gongzzang-db --lib access_log::tests::purpose_db_str
# Expected: ok. 1 passed (purpose_db_str does not call unimplemented)
cargo test -p gongzzang-db --lib access_log::tests::record_inserts_row -- --ignored
# Expected: panic — "not implemented: Step 6.2.4 에서 impl"
```

- [ ] **Step 6.2.4: Implement `record`**

Replace `unimplemented!()`:

```rust
    pub async fn record(
        &self,
        entry: AccessLogEntry,
        accessed_at: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO raw_vault_access_log
                (user_id, source, pnu, purpose, ticket_id, accessed_at, request_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id",
        )
        .bind(&entry.user_id)
        .bind(&entry.source)
        .bind(&entry.pnu)
        .bind(entry.purpose.as_db_str())
        .bind(&entry.ticket_id)
        .bind(accessed_at)
        .bind(&entry.request_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }
```

- [ ] **Step 6.2.5: Run — verify PASS (with test DB)**

```bash
cargo test -p gongzzang-db --lib access_log::tests -- --ignored
# Expected: 2 passed
```

- [ ] **Step 6.2.6: Commit**

```bash
git add crates/db/src/access_log.rs crates/db/src/lib.rs
git commit -m "feat(sp10-5-b-T6): PgVaultAccessLog::record (PIPA audit INSERT)"
```

---


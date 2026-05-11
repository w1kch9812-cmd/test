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

## Step 6.3: Admin endpoint handler (TDD with axum-test)

- [ ] **Step 6.3.1: Create `services/api/src/routes/admin/mod.rs`**

```rust
//! Admin-only routes — ZITADEL admin role 필수.

pub mod raw_vault;
```

- [ ] **Step 6.3.2: Create `services/api/src/routes/admin/raw_vault.rs` with failing test (handler signature ONLY)**

```rust
//! `GET /api/admin/raw_vault/:source/:pnu` — PIPA-compliant vault access.
//!
//! Spec §6.4 SSOT. Required: ZITADEL admin role + purpose enum + ticket_id.
//! audit log INSERT *전* 에 vault SELECT 안 됨 — fail-fast.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use gongzzang_db::{AccessLogEntry, AccessPurpose, PgVaultAccessLog};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct VaultQuery {
    pub purpose: String,
    pub ticket_id: String,
}

#[derive(Debug, Serialize)]
pub struct VaultResponse {
    pub source: String,
    pub pnu: String,
    pub captured_at: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
    pub message: String,
}

/// Path: /api/admin/raw_vault/:source/:pnu
/// Query: purpose, ticket_id
/// Auth: ZITADEL Bearer JWT with admin role
pub async fn get_vault_handler(
    State(state): State<AdminState>,
    Path((source, pnu)): Path<(String, String)>,
    Query(q): Query<VaultQuery>,
    auth: AdminAuth, // 향후 extractor (Step 6.3.4)
) -> Result<Json<VaultResponse>, (StatusCode, Json<ErrorBody>)> {
    unimplemented!("Step 6.3.5 에서 impl")
}

#[derive(Clone)]
pub struct AdminState {
    pub pool: PgPool,
    pub kms: Arc<aws_sdk_kms::Client>,
    pub access_log: PgVaultAccessLog,
}

// Placeholder extractor — Step 6.3.4 에서 ZITADEL JWT validation 구현
pub struct AdminAuth {
    pub user_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_query_deserialize() {
        let q: VaultQuery = serde_urlencoded::from_str(
            "purpose=drift_diagnosis&ticket_id=TICKET-1",
        )
        .unwrap();
        assert_eq!(q.purpose, "drift_diagnosis");
        assert_eq!(q.ticket_id, "TICKET-1");
    }

    #[test]
    fn purpose_enum_parse() {
        let p = parse_purpose("drift_diagnosis").unwrap();
        assert_eq!(p, AccessPurpose::DriftDiagnosis);
        assert!(parse_purpose("invalid").is_err());
    }
}
```

- [ ] **Step 6.3.3: Run — verify FAIL (parse_purpose undefined, unimplemented handler)**

```bash
cargo test -p api --lib routes::admin::raw_vault::tests::vault_query_deserialize
# Expected: ok. 1 passed (test only checks serde — no handler call)
cargo test -p api --lib routes::admin::raw_vault::tests::purpose_enum_parse
# Expected: error[E0425]: cannot find function `parse_purpose`
```

- [ ] **Step 6.3.4: Implement `parse_purpose` + `AdminAuth` extractor**

Append to `raw_vault.rs`:

```rust
fn parse_purpose(s: &str) -> Result<AccessPurpose, String> {
    match s {
        "incident_investigation" => Ok(AccessPurpose::IncidentInvestigation),
        "drift_diagnosis" => Ok(AccessPurpose::DriftDiagnosis),
        "customer_request" => Ok(AccessPurpose::CustomerRequest),
        _ => Err(format!("invalid purpose: {s}")),
    }
}

#[async_trait::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AdminAuth
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorBody>);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // ZITADEL JWT validation — 기존 `crates/auth` 의 `decode_zitadel_jwt`
        // 헬퍼 재사용. 본 구현은 `Authorization: Bearer <jwt>` 헤더의 `roles`
        // claim 에 'admin' 포함 검증.
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorBody {
                        error: "missing_auth".to_string(),
                        message: "Authorization header required".to_string(),
                    }),
                )
            })?;
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorBody {
                        error: "invalid_auth_scheme".to_string(),
                        message: "Bearer scheme required".to_string(),
                    }),
                )
            })?;
        // Integration: `crates/auth::decode_zitadel_jwt` 호출. 검증: `roles` claim
        // 에 "admin" 포함, `sub` extract. 기존 ZITADEL middleware 와 동일 비밀키.
        let claims = crate::auth::decode_zitadel_jwt(token).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorBody {
                    error: "invalid_token".to_string(),
                    message: "JWT validation failed".to_string(),
                }),
            )
        })?;
        if !claims.roles.contains(&"admin".to_string()) {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ErrorBody {
                    error: "missing_admin_role".to_string(),
                    message: "admin role required for vault access".to_string(),
                }),
            ));
        }
        Ok(AdminAuth { user_id: claims.sub })
    }
}
```

- [ ] **Step 6.3.5: Implement `get_vault_handler`**

Replace `unimplemented!()`:

```rust
pub async fn get_vault_handler(
    State(state): State<AdminState>,
    Path((source, pnu)): Path<(String, String)>,
    Query(q): Query<VaultQuery>,
    auth: AdminAuth,
) -> Result<Json<VaultResponse>, (StatusCode, Json<ErrorBody>)> {
    // 1. Purpose validation (400 if invalid)
    let purpose = parse_purpose(&q.purpose).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorBody {
                error: "invalid_purpose".to_string(),
                message: e,
            }),
        )
    })?;

    // 2. Generate request_id (또는 헤더에서 추출 — 본 구현은 uuid)
    let request_id = uuid::Uuid::new_v4().to_string();

    // 3. Audit INSERT *전에* 모든 검증 — INSERT 실패 시 vault SELECT 차단
    let accessed_at = Utc::now();
    let entry = AccessLogEntry {
        user_id: auth.user_id.clone(),
        source: source.clone(),
        pnu: pnu.clone(),
        purpose,
        ticket_id: q.ticket_id.clone(),
        request_id: request_id.clone(),
    };
    state
        .access_log
        .record(entry, accessed_at)
        .await
        .map_err(|e| {
            tracing::error!(
                target: "admin.vault.audit_fail",
                error = %e,
                "audit INSERT failed — blocking vault access (fail-fast)"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorBody {
                    error: "audit_insert_failed".to_string(),
                    message: "vault access blocked: audit log unavailable".to_string(),
                }),
            )
        })?;

    // 4. RLS 우회 + vault SELECT (ciphertext_blob + kms_key_id)
    let mut tx = state.pool.begin().await.map_err(internal_err)?;
    sqlx::query("SET LOCAL app.role = 'admin'")
        .execute(&mut *tx)
        .await
        .map_err(internal_err)?;
    let row: Option<(Vec<u8>, String, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT ciphertext_blob, kms_key_id, captured_at
           FROM parcel_external_data_pii_vault
          WHERE pnu = $1 AND source = $2
          ORDER BY captured_at DESC LIMIT 1",
    )
    .bind(&pnu)
    .bind(&source)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal_err)?;
    tx.commit().await.map_err(internal_err)?;

    let (blob, kms_key_id, captured_at) = row.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorBody {
                error: "not_found".to_string(),
                message: format!("vault row not found: {source}/{pnu}"),
            }),
        )
    })?;

    // 5. KMS decrypt — envelope blob → DEK → AES-GCM decrypt
    let raw = decrypt_vault_blob(&state.kms, &kms_key_id, &blob)
        .await
        .map_err(internal_err)?;

    Ok(Json(VaultResponse {
        source,
        pnu,
        captured_at: captured_at.to_rfc3339(),
        raw,
    }))
}

fn internal_err<E: std::fmt::Display>(e: E) -> (StatusCode, Json<ErrorBody>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorBody {
            error: "internal".to_string(),
            message: e.to_string(),
        }),
    )
}

async fn decrypt_vault_blob(
    kms: &aws_sdk_kms::Client,
    kms_key_id: &str,
    blob: &[u8],
) -> Result<serde_json::Value, String> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

    // Unpack: enc_dek_len(4B BE) || enc_dek || iv(12B) || ciphertext
    if blob.len() < 4 + 12 {
        return Err("blob too short".to_string());
    }
    let enc_dek_len = u32::from_be_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
    if blob.len() < 4 + enc_dek_len + 12 {
        return Err("blob format error".to_string());
    }
    let enc_dek = &blob[4..4 + enc_dek_len];
    let iv = &blob[4 + enc_dek_len..4 + enc_dek_len + 12];
    let ciphertext = &blob[4 + enc_dek_len + 12..];

    // KMS Decrypt → plaintext DEK
    let dec = kms
        .decrypt()
        .ciphertext_blob(aws_sdk_kms::primitives::Blob::new(enc_dek))
        .key_id(kms_key_id)
        .send()
        .await
        .map_err(|e| format!("kms decrypt: {e}"))?;
    let dek = dec
        .plaintext()
        .ok_or_else(|| "kms returned no plaintext".to_string())?;
    let cipher = Aes256Gcm::new_from_slice(dek.as_ref())
        .map_err(|e| format!("aes-gcm init: {e}"))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(iv), ciphertext)
        .map_err(|e| format!("aes-gcm decrypt: {e}"))?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("json parse: {e}"))
}
```

- [ ] **Step 6.3.6: Run — verify PASS (parse_purpose tests)**

```bash
cargo test -p api --lib routes::admin::raw_vault::tests
# Expected: 2 passed (vault_query_deserialize, purpose_enum_parse)
```

- [ ] **Step 6.3.7: Add axum-test dev-dependency to services/api**

`services/api/Cargo.toml`:

```toml
[dev-dependencies]
axum-test = "15"
```

- [ ] **Step 6.3.8: Commit**

```bash
git add crates/data-clients/raw-capture/Cargo.toml services/api/Cargo.toml services/api/src/routes/admin/ Cargo.toml
git commit -m "feat(sp10-5-b-T6): admin raw_vault endpoint + ZITADEL RBAC + KMS decrypt"
```

---

## Step 6.4: Mount admin router in main.rs

- [ ] **Step 6.4.1: Modify `services/api/src/main.rs` — add admin route**

main.rs 의 axum router 빌더에 추가:

```rust
mod routes;

// ... (기존 router 빌더 위치)
let admin_state = routes::admin::raw_vault::AdminState {
    pool: pool.clone(),
    kms: kms_client.clone(),
    access_log: gongzzang_db::PgVaultAccessLog::new(pool.clone()),
};
let admin_router = axum::Router::new()
    .route(
        "/api/admin/raw_vault/:source/:pnu",
        axum::routing::get(routes::admin::raw_vault::get_vault_handler),
    )
    .with_state(admin_state);
let app = app.merge(admin_router);
```

- [ ] **Step 6.4.2: Verify build**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 6.4.3: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-5-b-T6): mount admin raw_vault router on /api/admin/*"
```

---

## Step 6.5: Integration test (TDD with axum-test)

- [ ] **Step 6.5.1: Append axum-test based integration tests to `raw_vault.rs`**

```rust
    use axum::Router;
    use axum_test::TestServer;

    fn test_app(pool: PgPool, kms: Arc<aws_sdk_kms::Client>) -> Router {
        let state = AdminState {
            pool: pool.clone(),
            kms,
            access_log: PgVaultAccessLog::new(pool),
        };
        Router::new()
            .route(
                "/api/admin/raw_vault/:source/:pnu",
                axum::routing::get(get_vault_handler),
            )
            .with_state(state)
    }

    fn admin_jwt() -> String {
        // 테스트용 JWT — production 은 ZITADEL 발급
        "Bearer test-admin-jwt-with-admin-role".to_string()
    }

    #[tokio::test]
    #[ignore = "requires test DB + KMS"]
    async fn missing_purpose_returns_400() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let kms_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let kms = Arc::new(aws_sdk_kms::Client::new(&kms_config));
        let server = TestServer::new(test_app(pool, kms)).unwrap();
        let response = server
            .get("/api/admin/raw_vault/data_go_kr_building/1111010100100010003")
            .add_header("authorization", admin_jwt())
            .add_query_param("ticket_id", "TICKET-1")
            // purpose 누락
            .await;
        assert_eq!(response.status_code(), 400);
    }

    #[tokio::test]
    #[ignore = "requires test DB + KMS + admin JWT"]
    async fn non_admin_returns_403() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let kms_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let kms = Arc::new(aws_sdk_kms::Client::new(&kms_config));
        let server = TestServer::new(test_app(pool, kms)).unwrap();
        let response = server
            .get("/api/admin/raw_vault/data_go_kr_building/1111010100100010003")
            .add_header("authorization", "Bearer test-user-jwt-no-admin-role")
            .add_query_param("purpose", "drift_diagnosis")
            .add_query_param("ticket_id", "TICKET-1")
            .await;
        assert_eq!(response.status_code(), 403);
    }

    #[tokio::test]
    #[ignore = "requires test DB + KMS + admin JWT + pre-populated vault"]
    async fn successful_access_creates_audit_log() {
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        // ... (vault row 사전 INSERT, KMS Decrypt 가능한 key_id 사용)
        // 응답 200 + raw_vault_access_log 새 row 1건 검증
    }
```

- [ ] **Step 6.5.2: Run tests (ignored unless test env)**

```bash
cargo test -p api --lib routes::admin -- --ignored
# Expected (with env): 3 passed
# Expected (without env): 3 ignored
```

- [ ] **Step 6.5.3: Commit**

```bash
git add services/api/src/routes/admin/raw_vault.rs
git commit -m "test(sp10-5-b-T6): admin raw_vault integration tests (axum-test)"
```

---

## Acceptance — T6 완료 기준

- [ ] `migrations/30015_raw_vault_access_log.sql` 적용 (7 columns + 3 indexes)
- [ ] `crates/db/src/access_log.rs` — `PgVaultAccessLog::record` impl + unit test PASS
- [ ] `services/api/src/routes/admin/raw_vault.rs` — handler impl (parse_purpose, AdminAuth extractor, KMS decrypt)
- [ ] `cargo test -p api --lib routes::admin -- --ignored` (test env 있을 시) — 3 passed (400/403/200 + audit)
- [ ] Admin route `/api/admin/raw_vault/:source/:pnu` mounted in main.rs
- [ ] audit INSERT 실패 시 vault SELECT 차단 (fail-fast) — 코드 path 검증

**다음 task:** [T7-integration-health.md](T7-integration-health.md) — services/api state.rs/lib.rs 분리 + ReadinessResponse + sp10_panel_endpoints.rs 재작성.

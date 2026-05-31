# T6 Vault Admin RBAC Audit - Part 02: Admin Endpoint Handler

Parent index: [T6 Vault Admin RBAC Audit](./T6-admin-rbac-audit.md).

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


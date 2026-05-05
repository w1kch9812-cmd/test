//! `UserRepository` `Postgres` 구현체 (spec § 5.1 — 18 필드 + `OCC` + `tracing`).
//!
//! SP5-iv: `save` 가 트랜잭션 안에서 `user` UPSERT + `audit_log` INSERT +
//! `outbox_event` INSERT 를 함께 수행 — `MutationContext` 의 actor/action/
//! events 매핑은 SP5-iii `PgAdminActionRepository` 와 동일한 패턴이에요.

// `PgUserRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::broker_license::BrokerLicense;
use shared_kernel::business_number::BusinessNumber;
use shared_kernel::email::Email;
use shared_kernel::id::{AuditLogMarker, ExternalAccountMarker, Id, OutboxEventMarker, UserMarker};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;
use user_domain::entity::{User, UserKind, UserRole};
use user_domain::repository::{RepoError, UserRepository};

use crate::error_map::map_sqlx_err;

/// `User` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `user` 컬럼을 일관되게 가져오기 위한 상수.
const ALL_USER_COLUMNS: &str = "id, zitadel_sub, email, phone_kr_hash, display_name, user_kind, \
    business_number, business_verified_at, \
    broker_license_number, broker_verified_at, \
    roles, nice_verified_at, marketing_consent_at, \
    created_at, updated_at, last_login_at, deleted_at, version";

/// `PgRow` 를 `User` 로 변환해요. 18 필드 모두 round-trip.
#[allow(clippy::too_many_lines)]
fn row_to_user(row: &PgRow) -> Result<User, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let zitadel_sub: String = row
        .try_get("zitadel_sub")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let email_str: String = row
        .try_get("email")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let phone_kr_hash: Option<String> = row
        .try_get("phone_kr_hash")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let display_name: String = row
        .try_get("display_name")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_kind_str: String = row
        .try_get("user_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let business_number_str: Option<String> = row
        .try_get("business_number")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let business_verified_at: Option<DateTime<Utc>> = row
        .try_get("business_verified_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let broker_license_str: Option<String> = row
        .try_get("broker_license_number")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let broker_verified_at: Option<DateTime<Utc>> = row
        .try_get("broker_verified_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let roles_strs: Vec<String> = row
        .try_get("roles")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let nice_verified_at: Option<DateTime<Utc>> = row
        .try_get("nice_verified_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let marketing_consent_at: Option<DateTime<Utc>> = row
        .try_get("marketing_consent_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let last_login_at: Option<DateTime<Utc>> = row
        .try_get("last_login_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let deleted_at: Option<DateTime<Utc>> = row
        .try_get("deleted_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<UserMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed id in DB: {e}")))?;
    let email = Email::try_new(&email_str)
        .map_err(|e| RepoError::Database(format!("malformed email in DB: {e}")))?;
    let user_kind = match user_kind_str.as_str() {
        "individual" => UserKind::Individual,
        "corporation" => UserKind::Corporation,
        other => {
            return Err(RepoError::Database(format!(
                "unexpected user_kind in DB: {other}"
            )));
        }
    };
    let business_number = business_number_str
        .map(|s| {
            BusinessNumber::try_new(&s)
                .map_err(|e| RepoError::Database(format!("malformed business_number in DB: {e}")))
        })
        .transpose()?;
    let broker_license_number = broker_license_str
        .map(|s| {
            BrokerLicense::try_new(&s)
                .map_err(|e| RepoError::Database(format!("malformed broker_license in DB: {e}")))
        })
        .transpose()?;

    let mut roles = Vec::with_capacity(roles_strs.len());
    for s in roles_strs {
        let r = match s.as_str() {
            "Buyer" => UserRole::Buyer,
            "Seller" => UserRole::Seller,
            "Broker" => UserRole::Broker,
            "Developer" => UserRole::Developer,
            "Enterprise" => UserRole::Enterprise,
            "Operator" => UserRole::Operator,
            "Admin" => UserRole::Admin,
            other => {
                return Err(RepoError::Database(format!(
                    "unexpected role in DB: {other}"
                )));
            }
        };
        roles.push(r);
    }

    Ok(User {
        id,
        zitadel_sub,
        email,
        phone_kr_hash,
        display_name,
        user_kind,
        business_number,
        business_verified_at,
        broker_license_number,
        broker_verified_at,
        roles,
        nice_verified_at,
        marketing_consent_at,
        created_at,
        updated_at,
        last_login_at,
        deleted_at,
        version,
    })
}

#[async_trait]
impl UserRepository for PgUserRepository {
    #[instrument(skip(self), fields(user_id = %id.as_str()))]
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError> {
        let sql = format!(
            r#"select {ALL_USER_COLUMNS} from "user" where id = $1 and deleted_at is null"#
        );
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_user).transpose()
    }

    #[instrument(skip(self))]
    async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError> {
        let sql = format!(
            r#"select {ALL_USER_COLUMNS} from "user" where zitadel_sub = $1 and deleted_at is null"#
        );
        let row = sqlx::query(&sql)
            .bind(sub)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_user).transpose()
    }

    #[instrument(skip(self, email))]
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError> {
        let sql = format!(
            r#"select {ALL_USER_COLUMNS} from "user" where email = $1 and deleted_at is null"#
        );
        let row = sqlx::query(&sql)
            .bind(email.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_user).transpose()
    }

    /// 트랜잭션 안에서 `user` UPSERT + `audit_log` + `outbox_event` 를 함께 기록.
    ///
    /// SP5-iv 패턴 (SP5-iii 의 `PgAdminActionRepository.insert` 와 동일):
    /// 1. `pool.begin()` 으로 트랜잭션 시작
    /// 2. `user` UPSERT (`OCC` — `version` 일치 시 `version + 1`)
    /// 3. `audit_log` INSERT (`resource_kind = 'user'`)
    /// 4. `ctx.events` 마다 `outbox_event` INSERT (`aggregate_kind = 'user'`)
    /// 5. `tx.commit()` — 어느 단계 실패든 자동 rollback (`tx` `Drop`)
    ///
    /// `MutationContext` 매핑:
    /// - `ctx.actor_id` → `audit_log.actor_id` (`None` → `NULL` 시스템 액션)
    /// - `ctx.action` → `audit_log.action`
    /// - `ctx.metadata` → `audit_log.after_state`
    /// - `ctx.client_ip` → `audit_log.ip_address` (`$N::inet`)
    /// - `ctx.user_agent` → `audit_log.user_agent`
    /// - `ctx.correlation_id` → `audit_log.correlation_id`
    /// - `ctx.occurred_at` → `audit_log.created_at` (`None` → `Utc::now()`)
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, user, ctx), fields(
        user_id = %user.id.as_str(),
        version = user.version,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save(&self, user: &User, ctx: MutationContext) -> Result<(), RepoError> {
        let kind_str = user.user_kind.as_str();
        let role_strs: Vec<&str> = user.roles.iter().copied().map(UserRole::as_str).collect();

        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. user UPSERT with OCC.
        let result = sqlx::query(
            r#"
            insert into "user" (
                id, zitadel_sub, email, phone_kr_hash, display_name, user_kind,
                business_number, business_verified_at,
                broker_license_number, broker_verified_at,
                roles, nice_verified_at, marketing_consent_at,
                created_at, updated_at, last_login_at, deleted_at, version
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            on conflict (id) do update set
                email = excluded.email,
                phone_kr_hash = excluded.phone_kr_hash,
                display_name = excluded.display_name,
                user_kind = excluded.user_kind,
                business_number = excluded.business_number,
                business_verified_at = excluded.business_verified_at,
                broker_license_number = excluded.broker_license_number,
                broker_verified_at = excluded.broker_verified_at,
                roles = excluded.roles,
                nice_verified_at = excluded.nice_verified_at,
                marketing_consent_at = excluded.marketing_consent_at,
                updated_at = excluded.updated_at,
                last_login_at = excluded.last_login_at,
                deleted_at = excluded.deleted_at,
                version = "user".version + 1
            where "user".version = $18
            "#,
        )
        .bind(user.id.as_str())
        .bind(&user.zitadel_sub)
        .bind(user.email.as_str())
        .bind(user.phone_kr_hash.as_ref())
        .bind(&user.display_name)
        .bind(kind_str)
        .bind(user.business_number.as_ref().map(BusinessNumber::as_str))
        .bind(user.business_verified_at)
        .bind(
            user.broker_license_number
                .as_ref()
                .map(BrokerLicense::as_str),
        )
        .bind(user.broker_verified_at)
        .bind(&role_strs)
        .bind(user.nice_verified_at)
        .bind(user.marketing_consent_at)
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.last_login_at)
        .bind(user.deleted_at)
        .bind(user.version)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            // ON CONFLICT path with version mismatch (no row updated). tx Drop → rollback.
            return Err(RepoError::Conflict);
        }

        // 2. audit_log INSERT — same tx.
        let audit_id = Id::<AuditLogMarker>::new();
        let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);
        sqlx::query(
            r"
            insert into audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state,
                ip_address, user_agent,
                correlation_id, created_at
            )
            values ($1, $2, $3, 'user', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(user.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. outbox_event INSERT for each ctx.events — same tx.
        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'user', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(user.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // 4. commit — failure → tx Drop → rollback.
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    /// `external_account` 에 `zitadel` 행 삽입. `ON CONFLICT DO NOTHING` — 멱등.
    #[instrument(skip(self), fields(user_id = %user_id.as_str()))]
    async fn link_zitadel_account(
        &self,
        user_id: &Id<UserMarker>,
        zitadel_sub: &str,
    ) -> Result<(), RepoError> {
        let ext_id = Id::<ExternalAccountMarker>::new();
        sqlx::query(
            r"
            INSERT INTO external_account (id, user_id, provider, external_id)
            VALUES ($1, $2, 'zitadel', $3)
            ON CONFLICT (provider, external_id) DO NOTHING
            ",
        )
        .bind(ext_id.as_str())
        .bind(user_id.as_str())
        .bind(zitadel_sub)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }
}

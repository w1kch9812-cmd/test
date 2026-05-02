//! `UserRepository` `Postgres` 구현체.

// `PgUserRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use user_domain::entity::{User, UserKind};
use user_domain::repository::{RepoError, UserRepository};

/// `User` `Aggregate`의 `Postgres` 저장소.
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

/// `PgRow`를 `User`로 변환하는 공용 헬퍼.
///
/// T8 expanded `User` to 18 fields. Walking Skeleton `SELECT`만 8개 필드를 가져와서
/// 나머지는 `None`/`empty`로 채워요. db 레이어 확장은 sub-project 5에서.
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
    let display_name: String = row
        .try_get("display_name")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_kind_str: String = row
        .try_get("user_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id_typed = Id::<UserMarker>::try_from_str(&id_str)
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

    Ok(User {
        id: id_typed,
        zitadel_sub,
        email,
        phone_kr_hash: None,
        display_name,
        user_kind,
        business_number: None,
        business_verified_at: None,
        broker_license_number: None,
        broker_verified_at: None,
        roles: Vec::new(),
        nice_verified_at: None,
        marketing_consent_at: None,
        created_at,
        updated_at,
        last_login_at: None,
        deleted_at: None,
        version,
    })
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError> {
        let row = sqlx::query(
            r#"
            select id, zitadel_sub, email, display_name, user_kind,
                   created_at, updated_at, version
            from "user"
            where id = $1 and deleted_at is null
            "#,
        )
        .bind(id.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

        row.as_ref().map(row_to_user).transpose()
    }

    async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError> {
        let row = sqlx::query(
            r#"
            select id, zitadel_sub, email, display_name, user_kind,
                   created_at, updated_at, version
            from "user"
            where zitadel_sub = $1 and deleted_at is null
            "#,
        )
        .bind(sub)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

        row.as_ref().map(row_to_user).transpose()
    }

    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError> {
        let row = sqlx::query(
            r#"
            select id, zitadel_sub, email, display_name, user_kind,
                   created_at, updated_at, version
            from "user"
            where email = $1 and deleted_at is null
            "#,
        )
        .bind(email.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

        row.as_ref().map(row_to_user).transpose()
    }

    async fn save(&self, user: &User) -> Result<(), RepoError> {
        let kind_str = match user.user_kind {
            UserKind::Individual => "individual",
            UserKind::Corporation => "corporation",
        };

        // INSERT or UPDATE with optimistic-lock check.
        // - On INSERT (first save): all DB-default fields use defaults (roles='{}', etc.)
        // - On UPDATE: enforce version match + bump.
        let result = sqlx::query(
            r#"
            insert into "user"
              (id, zitadel_sub, email, display_name, user_kind,
               created_at, updated_at, version)
            values
              ($1, $2, $3, $4, $5, $6, $7, $8)
            on conflict (id) do update set
                email = excluded.email,
                display_name = excluded.display_name,
                user_kind = excluded.user_kind,
                updated_at = excluded.updated_at,
                version = "user".version + 1
            where "user".version = $8
            "#,
        )
        .bind(user.id.as_str())
        .bind(&user.zitadel_sub)
        .bind(user.email.as_str())
        .bind(&user.display_name)
        .bind(kind_str)
        .bind(user.created_at)
        .bind(user.updated_at)
        .bind(user.version)
        .execute(&self.pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            // Either ON CONFLICT path with version mismatch (no row updated)
            // or no row inserted (rare).
            return Err(RepoError::Conflict);
        }

        Ok(())
    }
}

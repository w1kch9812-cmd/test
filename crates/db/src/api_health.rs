//! `PgHealthCheckRepository` — `api_health_check` 테이블 인프라 구현.
//!
//! SP7-iii 의 SSOT. `crates/operations/api-health` 의 trait 구현.

#![allow(clippy::module_name_repetitions)]

use std::str::FromStr;
use std::sync::Arc;

use api_health_domain::{
    HealthCheckRecord, HealthCheckRepository, HealthStatus, NewHealthCheck, RepoError,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

/// `api_health_check` 테이블에 대한 `Postgres` 구현.
#[derive(Clone)]
pub struct PgHealthCheckRepository {
    pool: Arc<PgPool>,
}

impl PgHealthCheckRepository {
    /// 새 [`PgHealthCheckRepository`].
    #[must_use]
    pub const fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

fn map_repo_error(e: &sqlx::Error) -> RepoError {
    match e {
        sqlx::Error::Database(db_err)
            if db_err.is_check_violation() || db_err.is_unique_violation() =>
        {
            RepoError::Integrity(format!("{e}"))
        }
        _ => RepoError::Database(format!("{e}")),
    }
}

#[allow(clippy::cast_sign_loss)]
fn row_to_record(row: &PgRow) -> Result<HealthCheckRecord, RepoError> {
    let status_str: String = row.try_get("status").map_err(|e| map_repo_error(&e))?;
    let status = HealthStatus::from_str(&status_str)
        .map_err(|e| RepoError::Integrity(format!("invalid status '{status_str}': {e}")))?;
    let http_code: Option<i16> = row.try_get("http_code").map_err(|e| map_repo_error(&e))?;
    let duration_ms: i32 = row.try_get("duration_ms").map_err(|e| map_repo_error(&e))?;

    Ok(HealthCheckRecord {
        id: row.try_get("id").map_err(|e| map_repo_error(&e))?,
        api_name: row.try_get("api_name").map_err(|e| map_repo_error(&e))?,
        checked_at: row
            .try_get::<DateTime<Utc>, _>("checked_at")
            .map_err(|e| map_repo_error(&e))?,
        status,
        // CHECK constraint duration_ms >= 0 + http_code 는 항상 0..=599 범위.
        http_code: http_code.map(|c| c as u16),
        error_detail: row
            .try_get("error_detail")
            .map_err(|e| map_repo_error(&e))?,
        cron_run: row.try_get("cron_run").map_err(|e| map_repo_error(&e))?,
        duration_ms: duration_ms as u32,
    })
}

#[async_trait]
impl HealthCheckRepository for PgHealthCheckRepository {
    #[instrument(skip(self, new), fields(api = %new.api_name, status = %new.status))]
    #[allow(clippy::cast_possible_wrap)]
    async fn record(&self, new: NewHealthCheck<'_>) -> Result<HealthCheckRecord, RepoError> {
        let row = sqlx::query(
            r"
            INSERT INTO api_health_check
                (api_name, status, http_code, error_detail, cron_run, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, api_name, checked_at, status, http_code,
                      error_detail, cron_run, duration_ms
            ",
        )
        .bind(new.api_name)
        .bind(new.status.as_str())
        .bind(new.http_code.map(|c| c as i16))
        .bind(new.error_detail)
        .bind(new.cron_run)
        .bind(new.duration_ms as i32)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_repo_error(&e))?;

        row_to_record(&row)
    }

    #[instrument(skip(self), fields(api = %api_name))]
    async fn is_n_cron_runs_failed(&self, api_name: &str, n: u32) -> Result<bool, RepoError> {
        // 최근 N 개 cron run 의 status — fail 가 모두 N 개여야 true.
        let row = sqlx::query(
            r"
            WITH recent_cron AS (
                SELECT status
                FROM api_health_check
                WHERE api_name = $1 AND cron_run = true
                ORDER BY checked_at DESC
                LIMIT $2
            )
            SELECT
                (SELECT COUNT(*) FROM recent_cron) AS total,
                (SELECT COUNT(*) FROM recent_cron WHERE status != 'success') AS failures
            ",
        )
        .bind(api_name)
        .bind(i64::from(n))
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_repo_error(&e))?;

        let total: i64 = row.try_get("total").map_err(|e| map_repo_error(&e))?;
        let failures: i64 = row.try_get("failures").map_err(|e| map_repo_error(&e))?;

        // N 개가 모두 모이고, 모두 failure 면 true.
        Ok(total == i64::from(n) && failures == i64::from(n))
    }

    #[instrument(skip(self), fields(api = %api_name))]
    async fn find_latest(&self, api_name: &str) -> Result<Option<HealthCheckRecord>, RepoError> {
        let row = sqlx::query(
            r"
            SELECT id, api_name, checked_at, status, http_code,
                   error_detail, cron_run, duration_ms
            FROM api_health_check
            WHERE api_name = $1
            ORDER BY checked_at DESC
            LIMIT 1
            ",
        )
        .bind(api_name)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_repo_error(&e))?;

        row.as_ref().map(row_to_record).transpose()
    }
}

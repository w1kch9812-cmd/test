//! `PgRawCapture` — `RawCapture` trait 의 `Postgres` 구현체.
//!
//! `parcel_external_data` 테이블 (마이그 `V003_05`) 에 UPSERT. 같은
//! `(pnu, source)` 재호출 시 `raw_response` + `fetched_at` 갱신.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use raw_capture_client::{RawCapture, RawCaptureError};
use sqlx::PgPool;
use tracing::instrument;

/// `RawCapture` 의 `Postgres` 구현체.
#[derive(Debug, Clone)]
pub struct PgRawCapture {
    pool: PgPool,
}

impl PgRawCapture {
    /// 새 [`PgRawCapture`].
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RawCapture for PgRawCapture {
    #[instrument(skip(self, raw), fields(pnu = %pnu, source = %source))]
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError> {
        sqlx::query(
            r"
            insert into parcel_external_data (pnu, source, raw_response, fetched_at, expires_at)
            values ($1, $2, $3, $4, NULL)
            on conflict (pnu, source) do update set
                raw_response = excluded.raw_response,
                fetched_at = excluded.fetched_at
            ",
        )
        .bind(pnu)
        .bind(source)
        .bind(raw)
        .bind(fetched_at)
        .execute(&self.pool)
        .await
        .map_err(|e| RawCaptureError::Sink(e.to_string()))?;
        Ok(())
    }
}

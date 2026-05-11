//! `PgRawCapture` — `RawCapture` trait 의 `Postgres` 구현체. **DEPRECATED**.
//!
//! ADR 0026 채택 후 dead code — production wire 는 `services/api/src/r2_raw_capture.rs`
//! 의 `R2RawCapture` (Bronze = R2). 본 모듈은 *기존 integration test 호환* 유지를
//! 위해 trait 만 만족시킴 (receipt 반환). 별도 cleanup PR 에서 본 파일 + 테스트 삭제 예정.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use raw_capture_client::{RawCapture, RawCaptureError, RawCaptureKind, RawCaptureReceipt};
use sqlx::PgPool;
use tracing::instrument;

/// `RawCapture` 의 `Postgres` 구현체. **DEPRECATED — ADR 0026 superseded by `R2RawCapture`**.
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
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        let raw_str = raw.to_string();
        let byte_size = i64::try_from(raw_str.len()).unwrap_or(i64::MAX);
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
        // DEPRECATED path — receipt 의 object_key 는 "pg::{source}/{pnu}" placeholder.
        // 실제 활성 wire (R2RawCapture) 의 receipt 와 구분 가능.
        Ok(RawCaptureReceipt {
            object_key: format!("pg::{source}/{pnu}"),
            byte_size,
            kind: RawCaptureKind::NoOp,
            stored_at: fetched_at,
        })
    }
}

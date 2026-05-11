//! `TrackedRawCapture` — `RawCapture` decorator. inner sink (R2 / `NoOp` 등) 호출 후
//! receipt 를 받아 `parcel_external_data` (R2 pointer 메타) UPSERT.
//!
//! ADR 0026 + migration 30010 의 dead schema 갭 close — 30010 컬럼
//! (`r2_object_key`, `raw_byte_size`) 를 *실제로* 채우는 active writer.
//!
//! # 책임 분리 (SSOT)
//!
//! - `R2RawCapture` = R2/디스크 *적재* 만 (단일 책임)
//! - `TrackedRawCapture` = DB *추적* 만 (single point of metadata write)
//! - readers (`building_reader.rs`, V-World) 는 합성된 `dyn RawCapture` 만 봄 — DB 모름
//!
//! # 멱등성
//!
//! `parcel_external_data (pnu, source)` PK 라 같은 (pnu, source) 가 두 번 capture 되면
//! UPSERT (last-write-wins). raw 자체는 R2 의 timestamped 키로 모든 시점 보존되므로
//! DB 메타는 *최신* 만 추적해도 충분 (역사 query 는 R2 list).
//!
//! # DB write 실패 정책
//!
//! best-effort — DB UPSERT 실패해도 raw 자체는 inner sink 가 보존했으므로 warn 후 정상
//! 진행. caller (best-effort capture pattern) 의 `if let Err` 분기는 *raw 자체* 손실만
//! 잡음.

#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use raw_capture_client::{RawCapture, RawCaptureError, RawCaptureReceipt};
use sqlx::PgPool;
use tracing::{instrument, warn};

/// `RawCapture` decorator — inner sink 호출 후 `parcel_external_data` 메타 UPSERT.
pub struct TrackedRawCapture {
    inner: Arc<dyn RawCapture>,
    pool: PgPool,
}

impl TrackedRawCapture {
    /// 새 [`TrackedRawCapture`] — inner sink + DB pool 합성.
    #[must_use]
    pub const fn new(inner: Arc<dyn RawCapture>, pool: PgPool) -> Self {
        Self { inner, pool }
    }
}

#[async_trait]
impl RawCapture for TrackedRawCapture {
    #[instrument(skip(self, raw), fields(pnu = %pnu, source = %source))]
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        // 1) 먼저 inner sink (R2 / NoOp 등) — 적재 성공 보장 후 DB write.
        let receipt = self.inner.capture(pnu, source, raw, fetched_at).await?;

        // 2) parcel_external_data UPSERT — 30010 컬럼 (r2_object_key, raw_byte_size) 채움.
        //    raw_response 컬럼은 NULL 으로 명시 — ADR 0026 (R2 가 SSOT).
        //    NOTE: migration 30010 적용 후에만 raw_response NULL 가능 — 미적용 시 NOT NULL
        //    제약으로 INSERT 실패할 것. 적용 검증은 별도 테스트.
        let upsert_result = sqlx::query(
            r"
            insert into parcel_external_data (
                pnu, source, raw_response, fetched_at, expires_at,
                r2_object_key, raw_byte_size
            )
            values ($1, $2, NULL::jsonb, $3, NULL, $4, $5)
            on conflict (pnu, source) do update set
                fetched_at = excluded.fetched_at,
                r2_object_key = excluded.r2_object_key,
                raw_byte_size = excluded.raw_byte_size
            ",
        )
        .bind(pnu)
        .bind(source)
        .bind(receipt.stored_at)
        .bind(&receipt.object_key)
        .bind(receipt.byte_size)
        .execute(&self.pool)
        .await;

        if let Err(db_err) = upsert_result {
            // DB write 실패 = raw 자체는 inner sink 에 안전 — 메타만 lost.
            // production 에서는 alert 가 떠야 함 (Sentry → 운영팀).
            warn!(
                event = "raw_capture.metadata.upsert_failed",
                pnu = %pnu,
                source = %source,
                object_key = %receipt.object_key,
                error = %db_err,
                "parcel_external_data UPSERT 실패 — raw 자체는 sink 보존됨, 메타만 lost"
            );
            // metadata write 실패는 best-effort — receipt 자체는 정상 반환.
            // 진짜 raw 손실 (R2 + fallback 둘 다 실패) 는 inner sink 가 이미 Err 반환했음.
        }

        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use raw_capture_client::{NoOpRawCapture, RawCaptureKind};

    /// inner = NoOp 일 때 receipt 의 object_key 가 `noop::` prefix.
    /// DB 호출은 mock 안 됨 — 실 DB 없이는 metadata UPSERT path 검증 불가 (별도 integration test).
    #[tokio::test]
    async fn inner_receipt_passthrough() {
        // PgPool 없이 기본 path 만 검증 — DB write 자체는 integration test.
        let inner: Arc<dyn RawCapture> = Arc::new(NoOpRawCapture::new());
        let receipt = inner
            .capture(
                "1111010100100010000",
                "vworld",
                &serde_json::json!({"k": "v"}),
                Utc::now(),
            )
            .await
            .expect("noop ok");
        assert_eq!(receipt.kind, RawCaptureKind::NoOp);
        assert!(receipt.object_key.starts_with("noop::vworld/"));
        assert!(receipt.byte_size > 0);
    }
}

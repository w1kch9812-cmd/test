//! `PgListingReportRepository` — `Postgres` 구현체. **No OCC** + transactional
//! `audit_log`/`outbox_event` 패턴 (SP5-iii T8).
//!
//! `ListingReport` 는 어드민 신고 처리 워크플로우라 동시 충돌이 드물어
//! `version` 컬럼을 두지 않아요. `save` 는 `INSERT … ON CONFLICT (id) DO UPDATE`
//! (조건 없음) 로 신규/업데이트를 모두 처리하고, 같은 트랜잭션 안에서
//! `audit_log` row 와 `MutationContext::events` 의 각 도메인 이벤트마다
//! `outbox_event` row 를 함께 `INSERT` 해 transactional 추적성을 보장해요.
//!
//! 흐름은 SP5-iii T5 [`crates/db/src/admin_action.rs`] 와 같지만 `ListingReport`
//! 만의 차이가 있어요:
//!
//! 1. `pool.begin()` 으로 트랜잭션 시작
//! 2. `INSERT … ON CONFLICT (id) DO UPDATE` 로 `listing_report` 저장 (no OCC)
//! 3. `audit_log` row `INSERT` (`resource_kind = 'listing_report'`)
//! 4. `ctx.events` 의 각 이벤트마다 `outbox_event` `INSERT`
//!    (`aggregate_kind = 'listing_report'`)
//! 5. `tx.commit()` — 어느 단계든 실패 시 자동 rollback (`tx` `Drop`)
//!
//! # Anonymous reporter
//!
//! `reporter_id` 는 `Option<Id<UserMarker>>` — `None` 이면 익명 신고로 기록돼요.
//! DB 에 `NULL` 로 들어가고, `find_*` 에서도 그대로 `None` 으로 복원돼요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use listing_report_domain::entity::ListingReport;
use listing_report_domain::reason::ListingReportReason;
use listing_report_domain::repository::{ListingReportRepository, RepoError};
use listing_report_domain::status::ListingReportStatus;
use shared_kernel::id::{
    AuditLogMarker, Id, ListingMarker, ListingReportMarker, OutboxEventMarker, UserMarker,
};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `ListingReport` Aggregate 의 `Postgres` 저장소.
///
/// `save` 는 no-OCC + transactional `audit_log`/`outbox_event` 패턴.
#[derive(Debug, Clone)]
pub struct PgListingReportRepository {
    pool: PgPool,
}

impl PgListingReportRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `listing_report` 컬럼을 일관되게 가져오기 위한 상수.
const REPORT_COLUMNS: &str = "id, listing_id, reporter_id, reason, detail, \
    status, handler_id, handler_note, created_at, resolved_at";

fn parse_reason(s: &str) -> Result<ListingReportReason, RepoError> {
    ListingReportReason::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected reason: {s}")))
}

fn parse_status(s: &str) -> Result<ListingReportStatus, RepoError> {
    ListingReportStatus::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected status: {s}")))
}

/// `PgRow` → [`ListingReport`] 변환. 10 컬럼 round-trip (`version` 없음).
fn row_to_report(row: &PgRow) -> Result<ListingReport, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_id_str: String = row
        .try_get("listing_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let reporter_id_str: Option<String> = row
        .try_get("reporter_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let reason_str: String = row
        .try_get("reason")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let detail: Option<String> = row
        .try_get("detail")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let handler_id_str: Option<String> = row
        .try_get("handler_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let handler_note: Option<String> = row
        .try_get("handler_note")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let resolved_at: Option<DateTime<Utc>> = row
        .try_get("resolved_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingReportMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed listing_report id: {e}")))?;
    let listing_id = Id::<ListingMarker>::try_from_str(listing_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed listing_id: {e}")))?;
    let reporter_id = reporter_id_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed reporter_id: {e}")))
        })
        .transpose()?;
    let handler_id = handler_id_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed handler_id: {e}")))
        })
        .transpose()?;
    let reason = parse_reason(&reason_str)?;
    let status = parse_status(&status_str)?;

    Ok(ListingReport {
        id,
        listing_id,
        reporter_id,
        reason,
        detail,
        status,
        handler_id,
        handler_note,
        created_at,
        resolved_at,
    })
}

#[async_trait]
impl ListingReportRepository for PgListingReportRepository {
    /// 트랜잭션 안에서 `listing_report` + `audit_log` + `outbox_event` 를 함께 저장.
    ///
    /// `INSERT … ON CONFLICT (id) DO UPDATE …` (조건 없음) 로 신규/업데이트 모두
    /// 항상 1행 적용. 버전 컬럼이 없어서 `rows_affected` 검사가 필요 없어요.
    /// 어느 단계든 실패하면 `tx` `Drop` 으로 자동 rollback — 일관 상태 유지.
    ///
    /// `MutationContext` 매핑 (T5/T6/T7 와 동일):
    /// - `ctx.actor_id` → `audit_log.actor_id` (`None` → `NULL`, 시스템 액션)
    /// - `ctx.action` → `audit_log.action`
    /// - `ctx.metadata` → `audit_log.after_state`
    /// - `ctx.client_ip` → `audit_log.ip_address` (`$N::inet` 캐스팅)
    /// - `ctx.user_agent` → `audit_log.user_agent`
    /// - `ctx.correlation_id` → `audit_log.correlation_id`
    /// - `ctx.occurred_at` → `audit_log.created_at` (`None` → `Utc::now()`)
    /// - `ctx.events` → 각 이벤트마다 `outbox_event` row 1개
    ///   (`aggregate_kind = 'listing_report'`)
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, report, ctx), fields(
        report_id = %report.id.as_str(),
        status = %report.status.as_db_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save(&self, report: &ListingReport, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. UPSERT listing_report — no OCC, no version 컬럼.
        //    INSERT 분기: 신규 row.
        //    UPDATE 분기: 기존 row — handler 처리 결과 (status/handler_*/resolved_at) 만 갱신.
        //    `listing_id`/`reporter_id`/`reason`/`detail`/`created_at` 은 immutable
        //    so DO UPDATE 절에 포함하지 않아요.
        sqlx::query(
            r"
            insert into listing_report (
                id, listing_id, reporter_id, reason, detail,
                status, handler_id, handler_note, created_at, resolved_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            on conflict (id) do update set
                status = excluded.status,
                handler_id = excluded.handler_id,
                handler_note = excluded.handler_note,
                resolved_at = excluded.resolved_at
            ",
        )
        .bind(report.id.as_str())
        .bind(report.listing_id.as_str())
        .bind(report.reporter_id.as_ref().map(Id::as_str))
        .bind(report.reason.as_db_str())
        .bind(report.detail.as_deref())
        .bind(report.status.as_db_str())
        .bind(report.handler_id.as_ref().map(Id::as_str))
        .bind(report.handler_note.as_deref())
        .bind(report.created_at)
        .bind(report.resolved_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 2. INSERT audit_log — 같은 tx, resource_kind = 'listing_report'
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
            values ($1, $2, $3, 'listing_report', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(report.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. INSERT outbox_event for each ctx.events — 같은 tx,
        //    aggregate_kind = 'listing_report'
        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'listing_report', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(report.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // 4. commit — 실패 시 자동 rollback (tx Drop)
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(report_id = %id.as_str()))]
    async fn find_by_id(
        &self,
        id: &Id<ListingReportMarker>,
    ) -> Result<Option<ListingReport>, RepoError> {
        let sql = format!("select {REPORT_COLUMNS} from listing_report where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_report).transpose()
    }

    #[instrument(skip(self), fields(limit))]
    async fn find_open(&self, limit: u32) -> Result<Vec<ListingReport>, RepoError> {
        // 미처리 (status `Open` + `Investigating`) 신고를 오래된 순 (`created_at` ASC)
        // 으로 어드민 워크큐 용도. 부분 인덱스 `listing_report_open_idx` 는
        // `status = 'open'` 만 커버하므로 `Investigating` 은 seq scan — 운영 신고
        // 볼륨이 작아 허용. 필요 시 추후 인덱스 보강 (spec FU 후보).
        let sql = format!(
            "select {REPORT_COLUMNS} from listing_report \
             where status in ('open', 'investigating') \
             order by created_at asc \
             limit $1"
        );
        let rows = sqlx::query(&sql)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_report).collect()
    }

    #[instrument(skip(self), fields(listing_id = %listing_id.as_str()))]
    async fn find_by_listing(
        &self,
        listing_id: &Id<ListingMarker>,
    ) -> Result<Vec<ListingReport>, RepoError> {
        let sql = format!(
            "select {REPORT_COLUMNS} from listing_report \
             where listing_id = $1 \
             order by created_at desc"
        );
        let rows = sqlx::query(&sql)
            .bind(listing_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_report).collect()
    }
}

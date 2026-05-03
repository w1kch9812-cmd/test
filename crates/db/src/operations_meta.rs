//! `PgOperationsMetaRepository` — `Postgres` 구현체. **No OCC** + transactional
//! `audit_log`/`outbox_event` 패턴 (SP5-iii T9).
//!
//! `FeaturedContent` + `SystemAlert` 두 Aggregate 를 한 trait 으로 묶어서 처리해요.
//! 둘 다 `version` 컬럼이 없어 OCC 가 필요 없고, `save_*` 는
//! `INSERT … ON CONFLICT (id) DO UPDATE` (조건 없음) 로 신규/업데이트를 모두 처리.
//! 같은 트랜잭션 안에서 `audit_log` row 와 `MutationContext::events` 의 각 도메인
//! 이벤트마다 `outbox_event` row 를 함께 `INSERT` 해 transactional 추적성을 보장해요.
//!
//! 흐름은 SP5-iii T8 [`crates/db/src/listing_report.rs`] 와 동일:
//!
//! 1. `pool.begin()` 으로 트랜잭션 시작
//! 2. `INSERT … ON CONFLICT (id) DO UPDATE` 로 row 저장 (no OCC)
//! 3. `audit_log` row `INSERT` (`resource_kind = 'featured_content'` 또는 `'system_alert'`)
//! 4. `ctx.events` 의 각 이벤트마다 `outbox_event` `INSERT`
//!    (`aggregate_kind = 'featured_content'` 또는 `'system_alert'`)
//! 5. `tx.commit()` — 어느 단계든 실패 시 자동 rollback (`tx` `Drop`)
//!
//! # `find_active_featured`
//!
//! `feature_kind = $1 AND starts_at <= $2 AND $2 < ends_at` 의 half-open
//! interval. weight 내림차순, tie-break `created_at` 오름차순.
//!
//! # `find_unacknowledged_alerts`
//!
//! `acknowledged_at IS NULL` 만 필터 후 severity 우선순위 (critical > error >
//! warning > info), tie-break `created_at` 내림차순. 부분 인덱스
//! `system_alert_unack_idx` 가 `acknowledged_at IS NULL` 조건을 커버해요.

#![allow(clippy::module_name_repetitions, clippy::too_many_lines)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use operations_meta_domain::alert::{SystemAlert, SystemAlertSeverity};
use operations_meta_domain::featured::{
    FeaturedContent, FeaturedContentFeatureKind, FeaturedContentTargetKind,
};
use operations_meta_domain::repository::{OperationsMetaRepository, RepoError};
use shared_kernel::id::{
    AuditLogMarker, FeaturedContentMarker, Id, OutboxEventMarker, SystemAlertMarker, UserMarker,
};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `FeaturedContent` + `SystemAlert` 두 Aggregate 의 `Postgres` 저장소.
///
/// `save_*` 는 no-OCC + transactional `audit_log`/`outbox_event` 패턴.
#[derive(Debug, Clone)]
pub struct PgOperationsMetaRepository {
    pool: PgPool,
}

impl PgOperationsMetaRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `featured_content` 컬럼을 일관되게 가져오기 위한 상수.
const FC_COLUMNS: &str = "id, target_kind, target_id, feature_kind, weight, \
    starts_at, ends_at, purchased_by, impression_count, click_count, created_at";

/// `select` 절에서 모든 `system_alert` 컬럼을 일관되게 가져오기 위한 상수.
const SA_COLUMNS: &str = "id, severity, source, title, detail, metadata, \
    acknowledged_at, acknowledged_by, resolved_at, created_at";

fn parse_target_kind(s: &str) -> Result<FeaturedContentTargetKind, RepoError> {
    FeaturedContentTargetKind::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected target_kind: {s}")))
}

fn parse_feature_kind(s: &str) -> Result<FeaturedContentFeatureKind, RepoError> {
    FeaturedContentFeatureKind::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected feature_kind: {s}")))
}

fn parse_severity(s: &str) -> Result<SystemAlertSeverity, RepoError> {
    SystemAlertSeverity::from_db_str(s)
        .ok_or_else(|| RepoError::Database(format!("unexpected severity: {s}")))
}

/// `PgRow` → [`FeaturedContent`] 변환. 11 컬럼 round-trip (`version` 없음).
fn row_to_featured(row: &PgRow) -> Result<FeaturedContent, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_kind_str: String = row
        .try_get("target_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_id: String = row
        .try_get("target_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let feature_kind_str: String = row
        .try_get("feature_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let weight: i32 = row
        .try_get("weight")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let starts_at: DateTime<Utc> = row
        .try_get("starts_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let ends_at: DateTime<Utc> = row
        .try_get("ends_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let purchased_by_str: Option<String> = row
        .try_get("purchased_by")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let impression_count: i64 = row
        .try_get("impression_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let click_count: i64 = row
        .try_get("click_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<FeaturedContentMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed featured_content id: {e}")))?;
    let target_kind = parse_target_kind(&target_kind_str)?;
    let feature_kind = parse_feature_kind(&feature_kind_str)?;
    let purchased_by = purchased_by_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed purchased_by: {e}")))
        })
        .transpose()?;

    Ok(FeaturedContent {
        id,
        target_kind,
        target_id,
        feature_kind,
        weight,
        starts_at,
        ends_at,
        purchased_by,
        impression_count,
        click_count,
        created_at,
    })
}

/// `PgRow` → [`SystemAlert`] 변환. 10 컬럼 round-trip (`version` 없음).
fn row_to_alert(row: &PgRow) -> Result<SystemAlert, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let severity_str: String = row
        .try_get("severity")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let source: String = row
        .try_get("source")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let title: String = row
        .try_get("title")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let detail: Option<String> = row
        .try_get("detail")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let metadata: serde_json::Value = row
        .try_get("metadata")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let acknowledged_at: Option<DateTime<Utc>> = row
        .try_get("acknowledged_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let acknowledged_by_str: Option<String> = row
        .try_get("acknowledged_by")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let resolved_at: Option<DateTime<Utc>> = row
        .try_get("resolved_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<SystemAlertMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed system_alert id: {e}")))?;
    let severity = parse_severity(&severity_str)?;
    let acknowledged_by = acknowledged_by_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed acknowledged_by: {e}")))
        })
        .transpose()?;

    Ok(SystemAlert {
        id,
        severity,
        source,
        title,
        detail,
        metadata,
        acknowledged_at,
        acknowledged_by,
        resolved_at,
        created_at,
    })
}

#[async_trait]
impl OperationsMetaRepository for PgOperationsMetaRepository {
    /// 트랜잭션 안에서 `featured_content` + `audit_log` + `outbox_event` 를 함께 저장.
    ///
    /// `INSERT … ON CONFLICT (id) DO UPDATE …` (조건 없음) 로 신규/업데이트 모두
    /// 항상 1행 적용. 버전 컬럼이 없어서 `rows_affected` 검사가 필요 없어요.
    /// 어느 단계든 실패하면 `tx` `Drop` 으로 자동 rollback — 일관 상태 유지.
    ///
    /// `MutationContext` 매핑 (T5/T6/T7/T8 와 동일):
    /// - `ctx.actor_id` → `audit_log.actor_id` (`None` → `NULL`, 시스템 액션)
    /// - `ctx.action` → `audit_log.action`
    /// - `ctx.metadata` → `audit_log.after_state`
    /// - `ctx.client_ip` → `audit_log.ip_address` (`$N::inet` 캐스팅)
    /// - `ctx.user_agent` → `audit_log.user_agent`
    /// - `ctx.correlation_id` → `audit_log.correlation_id`
    /// - `ctx.occurred_at` → `audit_log.created_at` (`None` → `Utc::now()`)
    /// - `ctx.events` → 각 이벤트마다 `outbox_event` row 1개
    ///   (`aggregate_kind = 'featured_content'`)
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, fc, ctx), fields(
        fc_id = %fc.id.as_str(),
        feature_kind = %fc.feature_kind.as_db_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save_featured(
        &self,
        fc: &FeaturedContent,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. UPSERT featured_content — no OCC, no version 컬럼.
        //    `created_at` 은 immutable 이라 DO UPDATE 절에 포함하지 않아요.
        sqlx::query(
            r"
            insert into featured_content (
                id, target_kind, target_id, feature_kind, weight,
                starts_at, ends_at, purchased_by,
                impression_count, click_count, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            on conflict (id) do update set
                target_kind = excluded.target_kind,
                target_id = excluded.target_id,
                feature_kind = excluded.feature_kind,
                weight = excluded.weight,
                starts_at = excluded.starts_at,
                ends_at = excluded.ends_at,
                purchased_by = excluded.purchased_by,
                impression_count = excluded.impression_count,
                click_count = excluded.click_count
            ",
        )
        .bind(fc.id.as_str())
        .bind(fc.target_kind.as_db_str())
        .bind(&fc.target_id)
        .bind(fc.feature_kind.as_db_str())
        .bind(fc.weight)
        .bind(fc.starts_at)
        .bind(fc.ends_at)
        .bind(fc.purchased_by.as_ref().map(Id::as_str))
        .bind(fc.impression_count)
        .bind(fc.click_count)
        .bind(fc.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 2. INSERT audit_log — 같은 tx, resource_kind = 'featured_content'
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
            values ($1, $2, $3, 'featured_content', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(fc.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. INSERT outbox_event for each ctx.events — 같은 tx,
        //    aggregate_kind = 'featured_content'
        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'featured_content', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(fc.id.as_str())
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

    #[instrument(skip(self), fields(fc_id = %id.as_str()))]
    async fn find_featured_by_id(
        &self,
        id: &Id<FeaturedContentMarker>,
    ) -> Result<Option<FeaturedContent>, RepoError> {
        let sql = format!("select {FC_COLUMNS} from featured_content where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_featured).transpose()
    }

    /// `feature_kind` 가 일치하고 `at` 이 `[starts_at, ends_at)` 반-닫힘 구간에 속하는
    /// row 만, weight 내림차순, tie-break `created_at` 오름차순으로 반환.
    /// 인덱스 `featured_active_idx (feature_kind, starts_at, ends_at)` 활용.
    #[instrument(skip(self), fields(feature_kind = %feature_kind.as_db_str()))]
    async fn find_active_featured(
        &self,
        feature_kind: FeaturedContentFeatureKind,
        at: DateTime<Utc>,
    ) -> Result<Vec<FeaturedContent>, RepoError> {
        let sql = format!(
            "select {FC_COLUMNS} from featured_content \
             where feature_kind = $1 and starts_at <= $2 and $2 < ends_at \
             order by weight desc, created_at asc"
        );
        let rows = sqlx::query(&sql)
            .bind(feature_kind.as_db_str())
            .bind(at)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_featured).collect()
    }

    /// 트랜잭션 안에서 `system_alert` + `audit_log` + `outbox_event` 를 함께 저장.
    /// `save_featured` 와 같은 패턴이지만 `resource_kind = 'system_alert'`
    /// (audit) 와 `aggregate_kind = 'system_alert'` (outbox) 로 매핑.
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, alert, ctx), fields(
        alert_id = %alert.id.as_str(),
        severity = %alert.severity.as_db_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save_alert(&self, alert: &SystemAlert, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. UPSERT system_alert — no OCC, no version 컬럼.
        //    `created_at` 은 immutable 이라 DO UPDATE 절에 포함하지 않아요.
        sqlx::query(
            r"
            insert into system_alert (
                id, severity, source, title, detail, metadata,
                acknowledged_at, acknowledged_by, resolved_at, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            on conflict (id) do update set
                severity = excluded.severity,
                source = excluded.source,
                title = excluded.title,
                detail = excluded.detail,
                metadata = excluded.metadata,
                acknowledged_at = excluded.acknowledged_at,
                acknowledged_by = excluded.acknowledged_by,
                resolved_at = excluded.resolved_at
            ",
        )
        .bind(alert.id.as_str())
        .bind(alert.severity.as_db_str())
        .bind(&alert.source)
        .bind(&alert.title)
        .bind(alert.detail.as_deref())
        .bind(&alert.metadata)
        .bind(alert.acknowledged_at)
        .bind(alert.acknowledged_by.as_ref().map(Id::as_str))
        .bind(alert.resolved_at)
        .bind(alert.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 2. INSERT audit_log — 같은 tx, resource_kind = 'system_alert'
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
            values ($1, $2, $3, 'system_alert', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(alert.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. INSERT outbox_event for each ctx.events — 같은 tx,
        //    aggregate_kind = 'system_alert'
        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'system_alert', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(alert.id.as_str())
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

    #[instrument(skip(self), fields(alert_id = %id.as_str()))]
    async fn find_alert_by_id(
        &self,
        id: &Id<SystemAlertMarker>,
    ) -> Result<Option<SystemAlert>, RepoError> {
        let sql = format!("select {SA_COLUMNS} from system_alert where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_alert).transpose()
    }

    /// `acknowledged_at IS NULL` 인 알림을 severity 우선순위 (critical > error >
    /// warning > info), tie-break `created_at` 내림차순으로 최대 `limit` 건 반환.
    /// 부분 인덱스 `system_alert_unack_idx (severity, created_at desc) where
    /// acknowledged_at is null` 가 본 쿼리를 직접 커버해요.
    #[instrument(skip(self), fields(limit))]
    async fn find_unacknowledged_alerts(&self, limit: u32) -> Result<Vec<SystemAlert>, RepoError> {
        // severity 텍스트 정렬 (`info` < `warning` < ...) 은 알파벳 순이라 의미 순서와
        // 다르므로 명시적인 `case` 매핑으로 critical (0) > error (1) > warning (2) >
        // info (3) 순서를 보장해요. tie-break 는 `created_at` 내림차순.
        let sql = format!(
            "select {SA_COLUMNS} from system_alert \
             where acknowledged_at is null \
             order by \
                 case severity \
                     when 'critical' then 0 \
                     when 'error' then 1 \
                     when 'warning' then 2 \
                     when 'info' then 3 \
                 end, \
                 created_at desc \
             limit $1"
        );
        let rows = sqlx::query(&sql)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_alert).collect()
    }
}

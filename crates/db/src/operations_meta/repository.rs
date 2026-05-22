#![allow(clippy::too_many_lines)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use operations_meta_domain::alert::SystemAlert;
use operations_meta_domain::featured::{FeaturedContent, FeaturedContentFeatureKind};
use operations_meta_domain::repository::{OperationsMetaRepository, RepoError};
use shared_kernel::id::{
    AuditLogMarker, FeaturedContentMarker, Id, OutboxEventMarker, SystemAlertMarker,
};
use shared_kernel::mutation::MutationContext;
use tracing::instrument;

use super::rows::{row_to_alert, row_to_featured, FC_COLUMNS, SA_COLUMNS};
use super::PgOperationsMetaRepository;
use crate::error_map::map_sqlx_err;

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

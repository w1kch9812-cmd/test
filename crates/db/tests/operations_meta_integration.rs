//! `PgOperationsMetaRepository` 통합 테스트 — no OCC + transactional
//! `audit_log`/`outbox_event` 패턴, 2 aggregate (FC + SA) (SP5-iii T9).
//!
//! 5 시나리오:
//! 1. `save_featured` (`INSERT`) — `featured_content` + `audit_log` 1행
//!    (`resource_kind = 'featured_content'`)
//! 2. `find_active_featured` — half-open `[starts_at, ends_at)` 시간창 필터
//!    (활성 1건, 미래 1건 → 활성만 반환)
//! 3. `save_alert` with `metadata` — `audit_log.after_state` 가
//!    `MutationContext::metadata` 와 일치
//! 4. `find_unacknowledged_alerts` — acknowledge 된 알림 제외 +
//!    severity (critical > warning > info) 순서
//! 5. `save_alert` with no events — `outbox_event` 0건

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use chrono::{Duration, Utc};
use db::operations_meta::PgOperationsMetaRepository;
use db::user::PgUserRepository;
use operations_meta_domain::alert::{SystemAlert, SystemAlertSeverity};
use operations_meta_domain::featured::{
    FeaturedContent, FeaturedContentFeatureKind, FeaturedContentTargetKind,
};
use operations_meta_domain::repository::OperationsMetaRepository;
use serde_json::json;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};

/// `featured_content.purchased_by` / `system_alert.acknowledged_by` `FK` 용 admin 시드.
async fn seed_admin(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let admin = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Admin",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let admin_id = admin.id.clone();
    repo.save(&admin, test_ctx()).await.unwrap();
    admin_id
}

fn make_featured(
    target_id: &str,
    feature_kind: FeaturedContentFeatureKind,
    weight: i32,
    starts_offset_secs: i64,
    duration_secs: i64,
) -> FeaturedContent {
    let now = Utc::now();
    let starts_at = now + Duration::seconds(starts_offset_secs);
    let ends_at = starts_at + Duration::seconds(duration_secs);
    FeaturedContent::try_new(
        FeaturedContentTargetKind::Listing,
        target_id,
        feature_kind,
        weight,
        starts_at,
        ends_at,
        None,
        now,
    )
    .expect("valid featured")
}

fn make_alert(severity: SystemAlertSeverity, source: &str) -> SystemAlert {
    SystemAlert::try_new(
        severity,
        source,
        "Test alert title",
        Some("alert detail"),
        json!({}),
        Utc::now(),
    )
    .expect("valid alert")
}

#[tokio::test]
async fn save_featured_inserts_featured_and_audit_in_one_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin = seed_admin(&pool, "zsub-meta-1", "meta1@example.com").await;
    let repo = PgOperationsMetaRepository::new(pool.clone());

    let fc = make_featured(
        "lst_test123",
        FeaturedContentFeatureKind::HomepageFeatured,
        10,
        -3600,  // started 1h ago
        86_400, // 24h window
    );
    let ctx = MutationContext::new_user_action(admin, "corr_01HXY8RRPT4F8S1L01", "create");
    repo.save_featured(&fc, ctx).await.expect("save_featured");

    // featured_content row 1 개
    let fc_count: (i64,) = sqlx::query_as("select count(*) from featured_content where id = $1")
        .bind(fc.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(fc_count.0, 1);

    // audit_log row 1 개 (resource_kind = 'featured_content')
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'featured_content' \
         and resource_id = $1",
    )
    .bind(fc.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    // outbox 0 개 (events 비어 있음)
    let outbox_count: (i64,) = sqlx::query_as("select count(*) from outbox_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count.0, 0);

    // round-trip 검증
    let fetched = repo
        .find_featured_by_id(&fc.id)
        .await
        .unwrap()
        .expect("present");
    assert_eq!(fetched.target_id, "lst_test123");
    assert_eq!(
        fetched.feature_kind,
        FeaturedContentFeatureKind::HomepageFeatured
    );
    assert_eq!(fetched.weight, 10);
    assert_eq!(fetched.impression_count, 0);
    assert_eq!(fetched.click_count, 0);
    assert!(fetched.purchased_by.is_none());
}

#[tokio::test]
async fn find_active_featured_filters_by_time_window() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin = seed_admin(&pool, "zsub-meta-2", "meta2@example.com").await;
    let repo = PgOperationsMetaRepository::new(pool.clone());

    // 활성 — `now - 1h .. now + 1h`
    let active = make_featured(
        "lst_active",
        FeaturedContentFeatureKind::HomepageFeatured,
        5,
        -3600,
        7200,
    );
    // 미래 — `now + 2h .. now + 3h`
    let future = make_featured(
        "lst_future",
        FeaturedContentFeatureKind::HomepageFeatured,
        5,
        7200,
        3600,
    );
    // 다른 feature_kind — 활성이지만 `find_active_featured(HomepageFeatured)` 에는
    // 포함되면 안 돼요.
    let other_kind = make_featured(
        "lst_other_kind",
        FeaturedContentFeatureKind::SearchTop,
        99,
        -3600,
        7200,
    );

    repo.save_featured(
        &active,
        MutationContext::new_user_action(admin.clone(), "corr_01HXY8RRPT4F8S1L02", "create"),
    )
    .await
    .unwrap();
    repo.save_featured(
        &future,
        MutationContext::new_user_action(admin.clone(), "corr_01HXY8RRPT4F8S1L03", "create"),
    )
    .await
    .unwrap();
    repo.save_featured(
        &other_kind,
        MutationContext::new_user_action(admin, "corr_01HXY8RRPT4F8S1L04", "create"),
    )
    .await
    .unwrap();

    let results = repo
        .find_active_featured(FeaturedContentFeatureKind::HomepageFeatured, Utc::now())
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].target_id, "lst_active");
}

#[tokio::test]
async fn save_alert_records_metadata_in_audit_after_state() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin = seed_admin(&pool, "zsub-meta-3", "meta3@example.com").await;
    let repo = PgOperationsMetaRepository::new(pool.clone());

    let alert = make_alert(SystemAlertSeverity::Error, "pipeline.parcel_sync");
    let metadata = json!({"reason": "vworld_timeout"});
    let ctx = MutationContext::new_user_action(admin, "corr_01HXY8RRPT4F8S1L05", "create")
        .with_metadata(metadata.clone());
    repo.save_alert(&alert, ctx).await.expect("save_alert");

    // audit_log.after_state 가 metadata 와 동일
    let after_state: Option<serde_json::Value> = sqlx::query_scalar(
        "select after_state from audit_log where resource_kind = 'system_alert' \
         and resource_id = $1",
    )
    .bind(alert.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after_state, Some(metadata));

    // round-trip — detail / metadata 보존
    let fetched = repo
        .find_alert_by_id(&alert.id)
        .await
        .unwrap()
        .expect("present");
    assert_eq!(fetched.severity, SystemAlertSeverity::Error);
    assert_eq!(fetched.source, "pipeline.parcel_sync");
    assert_eq!(fetched.detail.as_deref(), Some("alert detail"));
    assert!(fetched.acknowledged_at.is_none());
    assert!(fetched.resolved_at.is_none());
}

#[tokio::test]
async fn find_unacknowledged_alerts_excludes_acked_and_orders_by_severity() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin = seed_admin(&pool, "zsub-meta-4", "meta4@example.com").await;
    let repo = PgOperationsMetaRepository::new(pool.clone());

    // 3 alerts: warning, critical, info — DB 에는 어떤 순서로 들어가도
    // `find_unacknowledged_alerts` 가 critical → warning → info 로 반환해야 해요.
    for sev in [
        SystemAlertSeverity::Warning,
        SystemAlertSeverity::Critical,
        SystemAlertSeverity::Info,
    ] {
        let alert = make_alert(sev, "test_source");
        repo.save_alert(
            &alert,
            MutationContext::new_user_action(admin.clone(), "corr_01HXY8RRPT4F8S1L06", "create"),
        )
        .await
        .unwrap();
    }

    // 1개 acknowledge — info 만 직접 SQL 로 acknowledge
    sqlx::query(
        "update system_alert set acknowledged_at = now(), acknowledged_by = $1 \
         where severity = 'info'",
    )
    .bind(admin.as_str())
    .execute(&pool)
    .await
    .unwrap();

    let unacked = repo.find_unacknowledged_alerts(10).await.unwrap();
    assert_eq!(unacked.len(), 2);
    // critical 가 가장 먼저, 그다음 warning
    assert_eq!(unacked[0].severity, SystemAlertSeverity::Critical);
    assert_eq!(unacked[1].severity, SystemAlertSeverity::Warning);
}

#[tokio::test]
async fn save_alert_with_no_events_inserts_no_outbox() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin = seed_admin(&pool, "zsub-meta-5", "meta5@example.com").await;
    let repo = PgOperationsMetaRepository::new(pool.clone());

    let alert = make_alert(SystemAlertSeverity::Warning, "test");
    let ctx = MutationContext::new_user_action(admin, "corr_01HXY8RRPT4F8S1L07", "create");
    repo.save_alert(&alert, ctx).await.unwrap();

    // events 가 비었으므로 outbox 0 건
    let outbox_count: (i64,) = sqlx::query_as("select count(*) from outbox_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count.0, 0);

    // 그래도 audit_log 는 1 건 남아 있어요 (system_alert)
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'system_alert' and resource_id = $1",
    )
    .bind(alert.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

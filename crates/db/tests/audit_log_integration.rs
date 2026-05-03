//! `PgAuditLogRepository` 통합 테스트 — `INSERT` + 3 `find_*` + `V002` immutable
//! 트리거 차단 검증.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::similar_names)]
#![cfg(feature = "integration")]

mod common;

use std::net::{IpAddr, Ipv4Addr};

use audit_log_domain::entity::AuditLog;
use audit_log_domain::repository::AuditLogRepository;
use chrono::{Duration, Utc};
use db::audit_log::PgAuditLogRepository;
use shared_kernel::id::{AuditLogMarker, Id};

use common::{setup_test_pool, truncate_all};

fn make_log(action: &str, resource_id: &str, correlation_id: &str) -> AuditLog {
    AuditLog::try_new(
        Id::<AuditLogMarker>::new(),
        None,
        action,
        "test_resource",
        resource_id,
        None,
        None,
        Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        Some("test-agent".to_owned()),
        correlation_id,
        Utc::now(),
    )
    .expect("audit log")
}

#[tokio::test]
async fn insert_persists_audit_log_with_ip_round_trip() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool.clone());

    let log = make_log("create", "res-1", "corr-insert");
    repo.insert(&log).await.expect("insert");

    // raw count
    let count: (i64,) = sqlx::query_as("select count(*) from audit_log where id = $1")
        .bind(log.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);

    // round-trip via find_by_correlation_id (ip_address text-cast 검증)
    let logs = repo
        .find_by_correlation_id("corr-insert")
        .await
        .expect("find");
    assert_eq!(logs.len(), 1);
    assert_eq!(
        logs[0].ip_address,
        Some(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
    );
    assert_eq!(logs[0].user_agent.as_deref(), Some("test-agent"));
    assert_eq!(logs[0].action, "create");
}

#[tokio::test]
async fn find_by_resource_filters_and_limits() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool);

    repo.insert(&make_log("create", "res-A", "corr-A1"))
        .await
        .unwrap();
    repo.insert(&make_log("update", "res-A", "corr-A2"))
        .await
        .unwrap();
    repo.insert(&make_log("create", "res-B", "corr-B1"))
        .await
        .unwrap();

    let logs = repo
        .find_by_resource("test_resource", "res-A", 10)
        .await
        .expect("find_by_resource ok");
    assert_eq!(logs.len(), 2);

    let limited = repo
        .find_by_resource("test_resource", "res-A", 1)
        .await
        .expect("limit honored");
    assert_eq!(limited.len(), 1);
}

#[tokio::test]
async fn find_by_correlation_id_filters_correctly() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool);

    let log = make_log("approve", "res-X", "corr-XYZ");
    repo.insert(&log).await.unwrap();
    repo.insert(&make_log("approve", "res-Y", "corr-other"))
        .await
        .unwrap();

    let logs = repo.find_by_correlation_id("corr-XYZ").await.expect("ok");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].resource_id, "res-X");

    let none = repo
        .find_by_correlation_id("nonexistent-corr")
        .await
        .expect("ok");
    assert_eq!(none.len(), 0);
}

#[tokio::test]
async fn immutable_trigger_blocks_update_and_delete() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool.clone());

    let log = make_log("create", "res-immut", "corr-immut");
    repo.insert(&log).await.unwrap();

    // V002 immutable trigger 가 UPDATE 차단
    let update_result = sqlx::query("update audit_log set action = 'tampered' where id = $1")
        .bind(log.id.as_str())
        .execute(&pool)
        .await;
    assert!(
        update_result.is_err(),
        "audit_log UPDATE 가 V002 트리거로 차단되어야"
    );

    // V002 immutable trigger 가 DELETE 차단 (현재 user 가 audit_archiver 가 아니므로)
    let delete_result = sqlx::query("delete from audit_log where id = $1")
        .bind(log.id.as_str())
        .execute(&pool)
        .await;
    assert!(
        delete_result.is_err(),
        "audit_log DELETE 가 V002 트리거로 차단되어야"
    );

    // 데이터는 그대로 남아있음
    let count: (i64,) = sqlx::query_as("select count(*) from audit_log where id = $1")
        .bind(log.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn find_by_actor_filters_by_since_and_limit() {
    use shared_kernel::id::UserMarker;

    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAuditLogRepository::new(pool);

    let actor: Id<UserMarker> = Id::new();
    let other_actor: Id<UserMarker> = Id::new();
    let now = Utc::now();

    // 같은 actor 의 로그 2건 (now)
    let log1 = AuditLog::try_new(
        Id::<AuditLogMarker>::new(),
        Some(actor.clone()),
        "create",
        "test_resource",
        "res-1",
        None,
        None,
        None,
        None,
        "corr-actor-1",
        now,
    )
    .expect("log1");
    let log2 = AuditLog::try_new(
        Id::<AuditLogMarker>::new(),
        Some(actor.clone()),
        "update",
        "test_resource",
        "res-1",
        None,
        None,
        None,
        None,
        "corr-actor-2",
        now,
    )
    .expect("log2");
    // since 보다 이전 (1시간 전) — since 필터로 제외되어야
    let log_old = AuditLog::try_new(
        Id::<AuditLogMarker>::new(),
        Some(actor.clone()),
        "old",
        "test_resource",
        "res-1",
        None,
        None,
        None,
        None,
        "corr-actor-old",
        now - Duration::hours(2),
    )
    .expect("log_old");
    // 다른 actor 의 로그 — actor_id 필터로 제외되어야
    let log_other = AuditLog::try_new(
        Id::<AuditLogMarker>::new(),
        Some(other_actor),
        "create",
        "test_resource",
        "res-1",
        None,
        None,
        None,
        None,
        "corr-actor-other",
        now,
    )
    .expect("log_other");

    repo.insert(&log1).await.unwrap();
    repo.insert(&log2).await.unwrap();
    repo.insert(&log_old).await.unwrap();
    repo.insert(&log_other).await.unwrap();

    let since = now - Duration::hours(1);
    let logs = repo
        .find_by_actor(&actor, since, 10)
        .await
        .expect("find_by_actor ok");
    assert_eq!(logs.len(), 2, "since 와 actor_id 필터로 2건만 매칭");

    let limited = repo
        .find_by_actor(&actor, since, 1)
        .await
        .expect("limit honored");
    assert_eq!(limited.len(), 1);
}

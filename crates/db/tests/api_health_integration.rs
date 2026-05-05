//! `PgHealthCheckRepository` 통합 테스트 — 실 `Postgres` 사용.
//!
//! 8 시나리오:
//! 1. record happy path
//! 2. record + invalid status (CHECK constraint 검증, 직접 SQL)
//! 3. `is_n_cron_runs_failed`: 3 cron 모두 fail → true
//! 4. `is_n_cron_runs_failed`: 3 중 1 success → false
//! 5. `is_n_cron_runs_failed`: 수동 trigger 무시 (cron 만 카운트)
//! 6. `is_n_cron_runs_failed`: 데이터 부족 (1개) → false
//! 7. `find_latest`: 가장 최근 record
//! 8. `find_latest`: unknown api → None

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::Arc;

use api_health_domain::{HealthCheckRepository, HealthStatus, NewHealthCheck};
use db::api_health::PgHealthCheckRepository;

use common::{setup_test_pool, truncate_all};

#[tokio::test]
async fn record_success_inserts_row() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    let new = NewHealthCheck {
        api_name: "data_go_kr.getBrTitleInfo",
        status: HealthStatus::Success,
        http_code: Some(200),
        error_detail: None,
        cron_run: true,
        duration_ms: 1234,
    };
    let record = repo.record(new).await.expect("record");
    assert_eq!(record.api_name, "data_go_kr.getBrTitleInfo");
    assert_eq!(record.status, HealthStatus::Success);
    assert_eq!(record.http_code, Some(200));
    assert!(record.cron_run);
    assert_eq!(record.duration_ms, 1234);
    assert!(record.id > 0);
}

#[tokio::test]
async fn record_invalid_status_violates_check_constraint() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    // HealthStatus enum 우회 — DB CHECK constraint 만 검증
    let result = sqlx::query(
        "INSERT INTO api_health_check (api_name, status, cron_run, duration_ms)
         VALUES ('test', 'invalid_status', true, 1)",
    )
    .execute(&pool)
    .await;
    assert!(
        result.is_err(),
        "CHECK constraint 가 invalid status 거부해야 함"
    );
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_true_when_3_consecutive_failures() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    for status in [
        HealthStatus::Http5xx,
        HealthStatus::Timeout,
        HealthStatus::Http5xx,
    ] {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_false_when_one_success_in_3() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    for status in [
        HealthStatus::Http5xx,
        HealthStatus::Success,
        HealthStatus::Http5xx,
    ] {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: true,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn is_n_cron_runs_failed_ignores_manual_dispatch() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    // cron 1 fail, manual 2 fail, cron 1 fail = cron 만 보면 2 fail (2 < 3)
    let mixes = [
        (HealthStatus::Http5xx, true),
        (HealthStatus::Http5xx, false),
        (HealthStatus::Http5xx, false),
        (HealthStatus::Http5xx, true),
    ];
    for (status, cron) in mixes {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status,
            http_code: None,
            error_detail: None,
            cron_run: cron,
            duration_ms: 100,
        })
        .await
        .unwrap();
    }
    assert!(
        !repo.is_n_cron_runs_failed("test_api", 3).await.unwrap(),
        "cron 만 카운트하면 2 fail < 3"
    );
}

#[tokio::test]
async fn is_n_cron_runs_failed_returns_false_with_insufficient_data() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    repo.record(NewHealthCheck {
        api_name: "test_api",
        status: HealthStatus::Http5xx,
        http_code: None,
        error_detail: None,
        cron_run: true,
        duration_ms: 100,
    })
    .await
    .unwrap();
    // 1개만 있을 때 n=3 = false
    assert!(!repo.is_n_cron_runs_failed("test_api", 3).await.unwrap());
}

#[tokio::test]
async fn find_latest_returns_most_recent() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    for i in 0..3_u32 {
        repo.record(NewHealthCheck {
            api_name: "test_api",
            status: HealthStatus::Success,
            http_code: Some(200),
            error_detail: Some(&format!("call {i}")),
            cron_run: true,
            duration_ms: 100 * (i + 1),
        })
        .await
        .unwrap();
    }
    let latest = repo.find_latest("test_api").await.unwrap().expect("Some");
    assert_eq!(latest.duration_ms, 300); // 가장 최근 = i=2
    assert_eq!(latest.error_detail, Some("call 2".to_owned()));
}

#[tokio::test]
async fn find_latest_returns_none_for_unknown_api() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    let latest = repo.find_latest("never.recorded").await.unwrap();
    assert!(latest.is_none());
}

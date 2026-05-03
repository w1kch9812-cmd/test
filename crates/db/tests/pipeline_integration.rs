//! `PgPipelineRepository` 통합 테스트 — 2 Aggregate (`PipelineSchedule` + `PipelineRun`)
//! + 시스템 액션 (SP5-iii T10).
//!
//! 5 시나리오:
//! 1. `save_schedule` (INSERT) — `pipeline_schedule` + `audit_log` 1 행 (`resource_kind = 'pipeline_schedule'`)
//! 2. `save_run` + 시스템 액션 (`actor_id = NULL`) — schedule 시드 후 run INSERT
//! 3. `save_schedule` `OCC` 충돌 → [`RepoError::Conflict`] (version 불일치)
//! 4. `find_schedule_by_kind` — UNIQUE `pipeline_kind` 정확 매칭
//! 5. `find_active_runs` — `status = 'running'` 만 반환 (`success` 제외)

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use data_pipeline_control::repository::{PipelineRepository, RepoError};
use data_pipeline_control::run::PipelineRun;
use data_pipeline_control::schedule::PipelineSchedule;
use data_pipeline_control::status::RunStatus;
use data_pipeline_control::trigger_kind::TriggerKind;
use db::pipeline::PgPipelineRepository;
use shared_kernel::id::{Id, PipelineRunMarker, PipelineScheduleMarker};
use shared_kernel::mutation::MutationContext;

use common::{setup_test_pool, truncate_all};

fn make_schedule(kind: &str) -> PipelineSchedule {
    let now = Utc::now();
    PipelineSchedule::try_new(
        Id::<PipelineScheduleMarker>::new(),
        kind,
        "0 3 * * *",
        true,
        "Asia/Seoul",
        serde_json::json!({}),
        None, // next_run_at
        None, // updated_by (시스템 시드)
        now,
    )
    .expect("schedule")
}

fn make_run(schedule_id: Id<PipelineScheduleMarker>, correlation_id: &str) -> PipelineRun {
    let now = Utc::now();
    PipelineRun::try_new_started(
        Id::<PipelineRunMarker>::new(),
        schedule_id,
        TriggerKind::Schedule,
        None,
        correlation_id,
        now,
    )
    .expect("run")
}

#[tokio::test]
async fn save_schedule_inserts_with_audit() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgPipelineRepository::new(pool.clone());

    let schedule = make_schedule("parcel_sync");
    let ctx = MutationContext::new_system_action("corr_pls_create_001", "create");
    repo.save_schedule(&schedule, ctx)
        .await
        .expect("save_schedule");

    // pipeline_schedule row 1 개
    let s_count: (i64,) = sqlx::query_as("select count(*) from pipeline_schedule where id = $1")
        .bind(schedule.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(s_count.0, 1);

    // audit_log 1 행 (resource_kind = 'pipeline_schedule')
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'pipeline_schedule' \
         and resource_id = $1",
    )
    .bind(schedule.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    // outbox 0 (events 비어 있음)
    let outbox_count: (i64,) = sqlx::query_as("select count(*) from outbox_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count.0, 0);

    // round-trip
    let fetched = repo
        .find_schedule_by_id(&schedule.id)
        .await
        .unwrap()
        .expect("present");
    assert_eq!(fetched.pipeline_kind, "parcel_sync");
    assert_eq!(fetched.cron_expression, "0 3 * * *");
    assert!(fetched.enabled);
    assert_eq!(fetched.timezone, "Asia/Seoul");
    assert_eq!(fetched.version, 1);
    assert!(fetched.last_run_at.is_none());
    assert!(fetched.running_lock_acquired_at.is_none());
}

#[tokio::test]
async fn save_run_with_audit_and_no_actor() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgPipelineRepository::new(pool.clone());

    // schedule 시드 (pipeline_run.schedule_id FK 충족)
    let schedule = make_schedule("building_sync");
    repo.save_schedule(
        &schedule,
        MutationContext::new_system_action("corr_pls_create_002", "create"),
    )
    .await
    .unwrap();

    // run INSERT — 시스템 액션
    let run = make_run(schedule.id.clone(), "corr_plr_create_002");
    let ctx = MutationContext::new_system_action(run.correlation_id.clone(), "create");
    repo.save_run(&run, ctx).await.expect("save_run");

    // pipeline_run 1 행
    let r_count: (i64,) = sqlx::query_as("select count(*) from pipeline_run where id = $1")
        .bind(run.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(r_count.0, 1);

    // audit_log 의 actor_id NULL (시스템 액션) + resource_kind = 'pipeline_run'
    let null_actor: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'pipeline_run' and actor_id is null \
         and resource_id = $1",
    )
    .bind(run.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(null_actor.0, 1);

    // round-trip
    let fetched = repo
        .find_run_by_id(&run.id)
        .await
        .unwrap()
        .expect("present");
    assert_eq!(fetched.schedule_id.as_str(), schedule.id.as_str());
    assert!(matches!(fetched.status, RunStatus::Running));
    assert!(matches!(fetched.triggered_by, TriggerKind::Schedule));
    assert!(fetched.triggered_by_user.is_none());
    assert!(fetched.finished_at.is_none());
    assert_eq!(fetched.items_processed, 0);
    assert_eq!(fetched.items_changed, 0);
    assert_eq!(fetched.correlation_id, "corr_plr_create_002");
}

#[tokio::test]
async fn schedule_occ_version_mismatch_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgPipelineRepository::new(pool.clone());

    let mut schedule = make_schedule("test_kind");
    repo.save_schedule(
        &schedule,
        MutationContext::new_system_action("corr_pls_create_003", "create"),
    )
    .await
    .unwrap();

    // 도메인 버전을 잘못된 값으로 — DB 의 1 과 불일치 → Conflict
    schedule.version = 99;
    let err = repo
        .save_schedule(
            &schedule,
            MutationContext::new_system_action("corr_pls_update_003", "update"),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, RepoError::Conflict));

    // tx 자동 rollback — audit_log 는 첫 INSERT 1행만 (충돌 시도분 추가 안 됨)
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'pipeline_schedule' \
         and resource_id = $1",
    )
    .bind(schedule.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn find_schedule_by_kind_returns_correct_one() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgPipelineRepository::new(pool.clone());

    let s1 = make_schedule("kind_a");
    let s2 = make_schedule("kind_b");
    repo.save_schedule(
        &s1,
        MutationContext::new_system_action("corr_pls_a", "create"),
    )
    .await
    .unwrap();
    repo.save_schedule(
        &s2,
        MutationContext::new_system_action("corr_pls_b", "create"),
    )
    .await
    .unwrap();

    let found = repo
        .find_schedule_by_kind("kind_b")
        .await
        .unwrap()
        .expect("kind_b present");
    assert_eq!(found.id.as_str(), s2.id.as_str());
    assert_eq!(found.pipeline_kind, "kind_b");

    // 미존재 kind 는 None
    let none = repo.find_schedule_by_kind("missing_kind").await.unwrap();
    assert!(none.is_none());
}

#[tokio::test]
async fn find_active_runs_filters_by_running_status() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgPipelineRepository::new(pool.clone());

    // schedule 시드
    let schedule = make_schedule("active_test");
    repo.save_schedule(
        &schedule,
        MutationContext::new_system_action("corr_pls_active", "create"),
    )
    .await
    .unwrap();

    // 2 runs INSERT — 둘 다 status = 'running' (try_new_started 기본)
    let r1 = make_run(schedule.id.clone(), "corr_plr_active_1");
    let r2 = make_run(schedule.id.clone(), "corr_plr_active_2");
    repo.save_run(
        &r1,
        MutationContext::new_system_action("corr_plr_active_1", "create"),
    )
    .await
    .unwrap();
    repo.save_run(
        &r2,
        MutationContext::new_system_action("corr_plr_active_2", "create"),
    )
    .await
    .unwrap();

    // r2 만 직접 SQL 로 'success' 전환 (도메인 메서드는 mutation 컨텍스트 필요)
    sqlx::query("update pipeline_run set status = 'success', finished_at = now() where id = $1")
        .bind(r2.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    // find_active_runs — r1 (running) 만
    let active = repo.find_active_runs().await.unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id.as_str(), r1.id.as_str());
    assert!(matches!(active[0].status, RunStatus::Running));

    // find_recent_runs — 둘 다 (status 무관, schedule 별 최근 N)
    let recent = repo.find_recent_runs(&schedule.id, 10).await.unwrap();
    assert_eq!(recent.len(), 2);
}

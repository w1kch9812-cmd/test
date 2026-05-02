//! `PipelineSchedule` Aggregate 테스트 (entity 가 500 줄 임계 근접 — `#[path]` 분리).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{Duration, TimeZone, Utc};
use serde_json::json;
use shared_kernel::id::Id;

use super::{PipelineError, PipelineSchedule};

fn t0() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 5, 2, 3, 0, 0).single().expect("valid")
}

fn sample() -> PipelineSchedule {
    PipelineSchedule::try_new(
        Id::new(),
        "parcel_sync",
        "0 3 1 */3 *",
        true,
        "Asia/Seoul",
        json!({}),
        None,
        None,
        t0(),
    )
    .expect("valid")
}

#[test]
fn happy_path_initial_values() {
    let s = sample();
    assert_eq!(s.pipeline_kind, "parcel_sync");
    assert_eq!(s.cron_expression, "0 3 1 */3 *");
    assert!(s.enabled);
    assert_eq!(s.timezone, "Asia/Seoul");
    assert!(s.last_run_at.is_none());
    assert!(s.next_run_at.is_none());
    assert!(s.running_lock_acquired_at.is_none());
    assert!(s.running_worker_id.is_none());
    assert!(s.updated_by.is_none());
    assert_eq!(s.version, 1);
    assert_eq!(s.updated_at, t0());
    assert!(s.id.as_str().starts_with("pls_"));
}

#[test]
fn try_new_trims_pipeline_kind_and_cron_and_timezone() {
    let s = PipelineSchedule::try_new(
        Id::new(),
        "  parcel_sync  ",
        "  0 3 1 */3 *  ",
        true,
        "  Asia/Seoul  ",
        json!({}),
        None,
        None,
        t0(),
    )
    .expect("valid");
    assert_eq!(s.pipeline_kind, "parcel_sync");
    assert_eq!(s.cron_expression, "0 3 1 */3 *");
    assert_eq!(s.timezone, "Asia/Seoul");
}

#[test]
fn rejects_empty_pipeline_kind() {
    let err = PipelineSchedule::try_new(
        Id::new(),
        "   ",
        "0 3 * * *",
        true,
        "Asia/Seoul",
        json!({}),
        None,
        None,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::EmptyPipelineKind);
}

#[test]
fn rejects_pipeline_kind_over_50_chars() {
    let long = "X".repeat(51);
    let err = PipelineSchedule::try_new(
        Id::new(),
        &long,
        "0 3 * * *",
        true,
        "Asia/Seoul",
        json!({}),
        None,
        None,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::PipelineKindTooLong { actual: 51 });
}

#[test]
fn accepts_pipeline_kind_exactly_50_chars() {
    let exactly = "X".repeat(50);
    let s = PipelineSchedule::try_new(
        Id::new(),
        &exactly,
        "0 3 * * *",
        true,
        "Asia/Seoul",
        json!({}),
        None,
        None,
        t0(),
    )
    .expect("50 ok");
    assert_eq!(s.pipeline_kind.chars().count(), 50);
}

#[test]
fn rejects_empty_cron_expression() {
    let err = PipelineSchedule::try_new(
        Id::new(),
        "parcel_sync",
        "",
        true,
        "Asia/Seoul",
        json!({}),
        None,
        None,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::EmptyCronExpression);
}

#[test]
fn rejects_cron_expression_over_100_chars() {
    let long = "X".repeat(101);
    let err = PipelineSchedule::try_new(
        Id::new(),
        "parcel_sync",
        &long,
        true,
        "Asia/Seoul",
        json!({}),
        None,
        None,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::CronExpressionTooLong { actual: 101 });
}

#[test]
fn rejects_empty_timezone() {
    let err = PipelineSchedule::try_new(
        Id::new(),
        "parcel_sync",
        "0 3 * * *",
        true,
        "   ",
        json!({}),
        None,
        None,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::EmptyTimezone);
}

#[test]
fn rejects_timezone_over_50_chars() {
    let long = "X".repeat(51);
    let err = PipelineSchedule::try_new(
        Id::new(),
        "parcel_sync",
        "0 3 * * *",
        true,
        &long,
        json!({}),
        None,
        None,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::TimezoneTooLong { actual: 51 });
}

#[test]
fn enable_disable_toggles_and_bumps_version() {
    let mut s = sample();
    let by: Id<shared_kernel::id::UserMarker> = Id::new();
    let t1 = t0() + Duration::seconds(60);
    s.disable(Some(by.clone()), t1);
    assert!(!s.enabled);
    assert_eq!(s.version, 2);
    assert_eq!(s.updated_at, t1);
    assert_eq!(s.updated_by.as_ref(), Some(&by));

    let t2 = t1 + Duration::seconds(60);
    s.enable(Some(by.clone()), t2);
    assert!(s.enabled);
    assert_eq!(s.version, 3);
    assert_eq!(s.updated_at, t2);
}

#[test]
fn acquire_lock_sets_fields_and_does_not_bump_version() {
    let mut s = sample();
    let before_version = s.version;
    let t1 = t0() + Duration::seconds(10);
    s.acquire_lock("worker-1", t1).expect("valid");
    assert_eq!(s.running_lock_acquired_at, Some(t1));
    assert_eq!(s.running_worker_id.as_deref(), Some("worker-1"));
    assert_eq!(s.version, before_version, "lock metadata is not a domain change");
    assert_eq!(s.updated_at, t1);
}

#[test]
fn acquire_lock_rejects_empty_worker_id() {
    let mut s = sample();
    let err = s.acquire_lock("   ", t0()).unwrap_err();
    assert_eq!(err, PipelineError::EmptyWorkerId);
    assert!(s.running_lock_acquired_at.is_none());
    assert!(s.running_worker_id.is_none());
}

#[test]
fn acquire_lock_rejects_worker_id_over_50_chars() {
    let mut s = sample();
    let long = "X".repeat(51);
    let err = s.acquire_lock(&long, t0()).unwrap_err();
    assert_eq!(err, PipelineError::WorkerIdTooLong { actual: 51 });
}

#[test]
fn acquire_lock_accepts_worker_id_exactly_50_chars() {
    let mut s = sample();
    let exactly = "X".repeat(50);
    s.acquire_lock(&exactly, t0()).expect("50 ok");
    assert_eq!(s.running_worker_id.as_ref().map(|w| w.chars().count()), Some(50));
}

#[test]
fn release_lock_clears_fields_and_does_not_bump_version() {
    let mut s = sample();
    s.acquire_lock("worker-1", t0()).expect("valid");
    let before_version = s.version;
    let t1 = t0() + Duration::seconds(60);
    s.release_lock(t1);
    assert!(s.running_lock_acquired_at.is_none());
    assert!(s.running_worker_id.is_none());
    assert_eq!(s.version, before_version);
    assert_eq!(s.updated_at, t1);
}

#[test]
fn record_run_sets_last_run_at_without_version_bump() {
    let mut s = sample();
    let before_version = s.version;
    let t1 = t0() + Duration::seconds(120);
    s.record_run(t1);
    assert_eq!(s.last_run_at, Some(t1));
    assert_eq!(s.version, before_version);
    assert_eq!(s.updated_at, t1);
}

#[test]
fn update_config_replaces_value_and_bumps_version() {
    let mut s = sample();
    let by: Id<shared_kernel::id::UserMarker> = Id::new();
    let t1 = t0() + Duration::seconds(180);
    s.update_config(json!({"sido_whitelist": ["11", "26"]}), Some(by.clone()), t1);
    assert_eq!(s.config, json!({"sido_whitelist": ["11", "26"]}));
    assert_eq!(s.version, 2);
    assert_eq!(s.updated_at, t1);
    assert_eq!(s.updated_by.as_ref(), Some(&by));
}

#[test]
fn update_cron_replaces_expression_and_bumps_version() {
    let mut s = sample();
    let by: Id<shared_kernel::id::UserMarker> = Id::new();
    let t1 = t0() + Duration::seconds(240);
    let next = t1 + Duration::days(1);
    s.update_cron("0 4 * * *", Some(next), Some(by.clone()), t1)
        .expect("valid cron");
    assert_eq!(s.cron_expression, "0 4 * * *");
    assert_eq!(s.next_run_at, Some(next));
    assert_eq!(s.version, 2);
    assert_eq!(s.updated_at, t1);
    assert_eq!(s.updated_by.as_ref(), Some(&by));
}

#[test]
fn update_cron_rejects_empty() {
    let mut s = sample();
    let before_version = s.version;
    let before_cron = s.cron_expression.clone();
    let err = s.update_cron("   ", None, None, t0()).unwrap_err();
    assert_eq!(err, PipelineError::EmptyCronExpression);
    assert_eq!(s.version, before_version);
    assert_eq!(s.cron_expression, before_cron);
}

#[test]
fn update_cron_rejects_too_long() {
    let mut s = sample();
    let long = "X".repeat(101);
    let err = s.update_cron(&long, None, None, t0()).unwrap_err();
    assert_eq!(err, PipelineError::CronExpressionTooLong { actual: 101 });
}

#[test]
fn serde_roundtrip() {
    let mut s = sample();
    s.acquire_lock("worker-1", t0()).expect("valid");
    s.record_run(t0());
    let payload = serde_json::to_string(&s).expect("serialize");
    let back: PipelineSchedule = serde_json::from_str(&payload).expect("deserialize");
    assert_eq!(s, back);
}

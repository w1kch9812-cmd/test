//! `PipelineRun` Aggregate 테스트.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{Duration, TimeZone, Utc};
use serde_json::json;
use shared_kernel::id::Id;

use super::{PipelineError, PipelineRun};
use crate::status::RunStatus;
use crate::trigger_kind::TriggerKind;

fn t0() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 5, 2, 3, 0, 0).single().expect("valid")
}

fn sample_run() -> PipelineRun {
    PipelineRun::try_new_started(
        Id::new(),
        Id::new(),
        TriggerKind::Schedule,
        None,
        "corr_abc",
        t0(),
    )
    .expect("valid")
}

#[test]
fn try_new_started_initial_state() {
    let r = sample_run();
    assert_eq!(r.status, RunStatus::Running);
    assert_eq!(r.started_at, t0());
    assert!(r.finished_at.is_none());
    assert_eq!(r.items_processed, 0);
    assert_eq!(r.items_changed, 0);
    assert_eq!(r.output_hashes, json!({}));
    assert!(r.error_message.is_none());
    assert_eq!(r.triggered_by, TriggerKind::Schedule);
    assert!(r.triggered_by_user.is_none());
    assert_eq!(r.correlation_id, "corr_abc");
    assert!(r.log_url.is_none());
    assert_eq!(r.steps, json!([]));
    assert!(r.id.as_str().starts_with("plr_"));
}

#[test]
fn try_new_started_trims_correlation_id() {
    let r = PipelineRun::try_new_started(
        Id::new(),
        Id::new(),
        TriggerKind::Schedule,
        None,
        "  corr_abc  ",
        t0(),
    )
    .expect("valid");
    assert_eq!(r.correlation_id, "corr_abc");
}

#[test]
fn try_new_started_rejects_empty_correlation_id() {
    let err = PipelineRun::try_new_started(
        Id::new(),
        Id::new(),
        TriggerKind::Schedule,
        None,
        "   ",
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::EmptyCorrelationId);
}

#[test]
fn try_new_started_rejects_correlation_id_over_30_chars() {
    let long = "X".repeat(31);
    let err = PipelineRun::try_new_started(
        Id::new(),
        Id::new(),
        TriggerKind::Schedule,
        None,
        &long,
        t0(),
    )
    .unwrap_err();
    assert_eq!(err, PipelineError::CorrelationIdTooLong { actual: 31 });
}

#[test]
fn try_new_started_accepts_correlation_id_exactly_30_chars() {
    let exactly = "X".repeat(30);
    let r = PipelineRun::try_new_started(
        Id::new(),
        Id::new(),
        TriggerKind::Schedule,
        None,
        &exactly,
        t0(),
    )
    .expect("30 ok");
    assert_eq!(r.correlation_id.chars().count(), 30);
}

#[test]
fn add_step_appends_running_entry() {
    let mut r = sample_run();
    r.add_step(
        "fetch_vworld",
        json!({"order": 1, "label": "V-World API fetch"}),
        t0(),
    )
    .expect("valid");
    let steps = r.steps.as_array().expect("array");
    assert_eq!(steps.len(), 1);
    let entry = steps[0].as_object().expect("object");
    assert_eq!(entry["name"], "fetch_vworld");
    assert_eq!(entry["status"], "running");
    assert_eq!(entry["order"], 1);
    assert_eq!(entry["label"], "V-World API fetch");
    assert!(entry.contains_key("started_at"));
}

#[test]
fn add_step_rejects_empty_name() {
    let mut r = sample_run();
    let err = r.add_step("   ", json!({}), t0()).unwrap_err();
    assert_eq!(err, PipelineError::EmptyStepName);
}

#[test]
fn complete_step_marks_success_and_accumulates_counters() {
    let mut r = sample_run();
    r.add_step("fetch", json!({"order": 1}), t0()).expect("valid");
    let t1 = t0() + Duration::seconds(60);
    r.complete_step(
        "fetch",
        100,
        25,
        Some(("sido_11".to_owned(), "abc123".to_owned())),
        t1,
    )
    .expect("valid");
    let steps = r.steps.as_array().expect("array");
    assert_eq!(steps[0]["status"], "success");
    assert!(steps[0].as_object().unwrap().contains_key("finished_at"));
    assert_eq!(r.items_processed, 100);
    assert_eq!(r.items_changed, 25);
    assert_eq!(r.output_hashes["sido_11"], "abc123");
}

#[test]
fn complete_step_returns_not_found_for_missing_step() {
    let mut r = sample_run();
    let err = r
        .complete_step("missing", 10, 5, None, t0())
        .unwrap_err();
    assert!(matches!(err, PipelineError::StepNotFound(s) if s == "missing"));
}

#[test]
fn fail_step_marks_failed_with_error_but_does_not_change_run_status() {
    let mut r = sample_run();
    r.add_step("fetch", json!({"order": 1}), t0()).expect("valid");
    let t1 = t0() + Duration::seconds(30);
    r.fail_step("fetch", "timeout", t1).expect("valid");
    let entry = r.steps.as_array().unwrap()[0].as_object().unwrap();
    assert_eq!(entry["status"], "failed");
    assert_eq!(entry["error"], "timeout");
    assert!(entry.contains_key("finished_at"));
    // run-level status untouched.
    assert_eq!(r.status, RunStatus::Running);
}

#[test]
fn fail_step_returns_not_found_for_missing_step() {
    let mut r = sample_run();
    let err = r.fail_step("missing", "boom", t0()).unwrap_err();
    assert!(matches!(err, PipelineError::StepNotFound(s) if s == "missing"));
}

#[test]
fn complete_run_sets_success_and_finished_at() {
    let mut r = sample_run();
    let t1 = t0() + Duration::seconds(120);
    r.complete_run(t1).expect("valid");
    assert_eq!(r.status, RunStatus::Success);
    assert_eq!(r.finished_at, Some(t1));
}

#[test]
fn complete_run_immutable_after_terminal() {
    let mut r = sample_run();
    r.complete_run(t0()).expect("valid");
    let err = r.complete_run(t0() + Duration::seconds(60)).unwrap_err();
    assert_eq!(err, PipelineError::AlreadyTerminal("success"));
}

#[test]
fn add_step_rejected_after_terminal() {
    let mut r = sample_run();
    r.complete_run(t0()).expect("valid");
    let err = r
        .add_step("late", json!({}), t0() + Duration::seconds(10))
        .unwrap_err();
    assert_eq!(err, PipelineError::AlreadyTerminal("success"));
}

#[test]
fn complete_step_rejected_after_terminal() {
    let mut r = sample_run();
    r.add_step("fetch", json!({"order": 1}), t0()).expect("valid");
    r.complete_run(t0() + Duration::seconds(60)).expect("valid");
    let err = r
        .complete_step("fetch", 1, 1, None, t0() + Duration::seconds(120))
        .unwrap_err();
    assert_eq!(err, PipelineError::AlreadyTerminal("success"));
}

#[test]
fn fail_step_rejected_after_terminal() {
    let mut r = sample_run();
    r.add_step("fetch", json!({"order": 1}), t0()).expect("valid");
    r.fail_run("boom", t0() + Duration::seconds(60)).expect("valid");
    let err = r
        .fail_step("fetch", "late", t0() + Duration::seconds(120))
        .unwrap_err();
    assert_eq!(err, PipelineError::AlreadyTerminal("failed"));
}

#[test]
fn complete_run_skipped_unchanged_sets_status() {
    let mut r = sample_run();
    let t1 = t0() + Duration::seconds(60);
    r.complete_run_skipped_unchanged(t1).expect("valid");
    assert_eq!(r.status, RunStatus::SkippedUnchanged);
    assert_eq!(r.finished_at, Some(t1));
}

#[test]
fn fail_run_sets_failed_and_error_message() {
    let mut r = sample_run();
    let t1 = t0() + Duration::seconds(60);
    r.fail_run("upstream timeout", t1).expect("valid");
    assert_eq!(r.status, RunStatus::Failed);
    assert_eq!(r.error_message.as_deref(), Some("upstream timeout"));
    assert_eq!(r.finished_at, Some(t1));
}

#[test]
fn fail_run_rejects_error_message_over_2000_chars() {
    let mut r = sample_run();
    let long = "X".repeat(2001);
    let err = r.fail_run(&long, t0()).unwrap_err();
    assert_eq!(err, PipelineError::ErrorMessageTooLong { actual: 2001 });
    assert_eq!(r.status, RunStatus::Running, "no mutation on validation failure");
}

#[test]
fn fail_run_accepts_error_message_exactly_2000_chars() {
    let mut r = sample_run();
    let exactly = "X".repeat(2000);
    r.fail_run(&exactly, t0()).expect("2000 ok");
    assert_eq!(r.status, RunStatus::Failed);
}

#[test]
fn fail_run_rejected_after_terminal() {
    let mut r = sample_run();
    r.complete_run(t0()).expect("valid");
    let err = r.fail_run("late", t0() + Duration::seconds(60)).unwrap_err();
    assert_eq!(err, PipelineError::AlreadyTerminal("success"));
}

#[test]
fn abort_run_sets_aborted() {
    let mut r = sample_run();
    let t1 = t0() + Duration::seconds(60);
    r.abort_run(t1).expect("valid");
    assert_eq!(r.status, RunStatus::Aborted);
    assert_eq!(r.finished_at, Some(t1));
}

#[test]
fn abort_run_rejected_after_terminal() {
    let mut r = sample_run();
    r.complete_run_skipped_unchanged(t0()).expect("valid");
    let err = r.abort_run(t0() + Duration::seconds(60)).unwrap_err();
    assert_eq!(err, PipelineError::AlreadyTerminal("skipped_unchanged"));
}

#[test]
fn set_log_url_accepts_500_chars() {
    let mut r = sample_run();
    let exactly = "X".repeat(500);
    r.set_log_url(&exactly).expect("500 ok");
    assert_eq!(r.log_url.as_ref().map(|u| u.chars().count()), Some(500));
}

#[test]
fn set_log_url_rejects_over_500_chars() {
    let mut r = sample_run();
    let long = "X".repeat(501);
    let err = r.set_log_url(&long).unwrap_err();
    assert_eq!(err, PipelineError::LogUrlTooLong { actual: 501 });
}

#[test]
fn serde_roundtrip_running() {
    let r = sample_run();
    let payload = serde_json::to_string(&r).expect("serialize");
    let back: PipelineRun = serde_json::from_str(&payload).expect("deserialize");
    assert_eq!(r, back);
}

#[test]
fn serde_roundtrip_after_terminal() {
    let mut r = sample_run();
    r.add_step("fetch", json!({"order": 1}), t0()).expect("valid");
    r.complete_step(
        "fetch",
        50,
        10,
        Some(("sido_11".to_owned(), "abc".to_owned())),
        t0() + Duration::seconds(30),
    )
    .expect("valid");
    r.complete_run(t0() + Duration::seconds(60)).expect("valid");
    let payload = serde_json::to_string(&r).expect("serialize");
    let back: PipelineRun = serde_json::from_str(&payload).expect("deserialize");
    assert_eq!(r, back);
    assert_eq!(back.status, RunStatus::Success);
}

#[test]
fn add_step_handles_non_object_payload_by_starting_fresh() {
    let mut r = sample_run();
    r.add_step("fetch", json!(42), t0()).expect("valid");
    let entry = r.steps.as_array().unwrap()[0].as_object().unwrap();
    assert_eq!(entry["name"], "fetch");
    assert_eq!(entry["status"], "running");
}

#[test]
fn complete_step_with_no_output_hash_does_not_mutate_output_hashes() {
    let mut r = sample_run();
    r.add_step("fetch", json!({"order": 1}), t0()).expect("valid");
    let before = r.output_hashes.clone();
    r.complete_step("fetch", 5, 2, None, t0() + Duration::seconds(30))
        .expect("valid");
    assert_eq!(r.output_hashes, before);
}

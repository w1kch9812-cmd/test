//! `AuditLog` Aggregate 테스트 (entity 가 500 줄 임계 근접 — `#[path]` 분리).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::*;

fn sample_before() -> serde_json::Value {
    serde_json::json!({"status": "draft", "title": "before"})
}

fn sample_after() -> serde_json::Value {
    serde_json::json!({"status": "published", "title": "after"})
}

fn make_full(
    action: &str,
    resource_kind: &str,
    resource_id: &str,
    correlation_id: &str,
) -> Result<AuditLog, AuditLogError> {
    AuditLog::try_new(
        Id::new(),
        Some(Id::new()),
        action,
        resource_kind,
        resource_id,
        Some(sample_before()),
        Some(sample_after()),
        Some(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))),
        Some("Mozilla/5.0 (Test)".to_owned()),
        correlation_id,
        Utc::now(),
    )
}

#[test]
fn happy_path_full_fields_populated() {
    let log = make_full(
        "listing.published",
        "listing",
        "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G",
        "corr_01HXY3NK0Z9F6S1B2C3D4",
    )
    .expect("valid");
    assert_eq!(log.action, "listing.published");
    assert_eq!(log.resource_kind, "listing");
    assert_eq!(log.resource_id, "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G");
    assert!(log.actor_id.is_some());
    assert!(log.before_state.is_some());
    assert!(log.after_state.is_some());
    assert!(log.ip_address.is_some());
    assert!(log.user_agent.is_some());
}

#[test]
fn happy_path_system_action_actor_id_none() {
    let log = AuditLog::try_new(
        Id::new(),
        None, // system action
        "batch.cleanup_expired",
        "notification",
        "ntf_01HXY3NK0Z9F6S1B2C3D4E5F6G",
        None,
        None,
        None,
        None,
        "sys_01HXY3NK0Z9F6S1B2C3D4",
        Utc::now(),
    )
    .expect("valid");
    assert!(log.is_system_action());
    assert!(log.actor_id.is_none());
}

#[test]
fn rejects_empty_action() {
    let err = make_full("", "listing", "lst_x", "corr_x").unwrap_err();
    assert!(matches!(err, AuditLogError::EmptyAction));
}

#[test]
fn rejects_action_over_100_chars() {
    let long = "X".repeat(101);
    let err = make_full(&long, "listing", "lst_x", "corr_x").unwrap_err();
    assert!(matches!(err, AuditLogError::ActionTooLong { actual: 101 }));
}

#[test]
fn rejects_empty_resource_kind() {
    let err = make_full("listing.published", "", "lst_x", "corr_x").unwrap_err();
    assert!(matches!(err, AuditLogError::EmptyResourceKind));
}

#[test]
fn rejects_resource_kind_over_50_chars() {
    let long = "X".repeat(51);
    let err = make_full("listing.published", &long, "lst_x", "corr_x").unwrap_err();
    assert!(matches!(
        err,
        AuditLogError::ResourceKindTooLong { actual: 51 }
    ));
}

#[test]
fn rejects_empty_resource_id() {
    let err = make_full("listing.published", "listing", "", "corr_x").unwrap_err();
    assert!(matches!(err, AuditLogError::EmptyResourceId));
}

#[test]
fn rejects_resource_id_over_50_chars() {
    let long = "X".repeat(51);
    let err = make_full("listing.published", "listing", &long, "corr_x").unwrap_err();
    assert!(matches!(
        err,
        AuditLogError::ResourceIdTooLong { actual: 51 }
    ));
}

#[test]
fn rejects_empty_correlation_id() {
    let err = make_full("listing.published", "listing", "lst_x", "").unwrap_err();
    assert!(matches!(err, AuditLogError::EmptyCorrelationId));
}

#[test]
fn rejects_correlation_id_over_30_chars() {
    let long = "X".repeat(31);
    let err = make_full("listing.published", "listing", "lst_x", &long).unwrap_err();
    assert!(matches!(
        err,
        AuditLogError::CorrelationIdTooLong { actual: 31 }
    ));
}

#[test]
fn rejects_user_agent_over_500_chars() {
    let long_ua = "X".repeat(501);
    let err = AuditLog::try_new(
        Id::new(),
        Some(Id::new()),
        "listing.published",
        "listing",
        "lst_x",
        None,
        None,
        None,
        Some(long_ua),
        "corr_x",
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        AuditLogError::UserAgentTooLong { actual: 501 }
    ));
}

#[test]
fn is_system_action_true_when_actor_none() {
    let log = AuditLog::try_new(
        Id::new(),
        None,
        "system.tick",
        "system",
        "tick",
        None,
        None,
        None,
        None,
        "corr_sys",
        Utc::now(),
    )
    .expect("valid");
    assert!(log.is_system_action());
}

#[test]
fn is_system_action_false_when_actor_some() {
    let log = make_full("listing.published", "listing", "lst_x", "corr_x").expect("valid");
    assert!(!log.is_system_action());
}

#[test]
fn serde_roundtrip_full() {
    let log = make_full(
        "listing.published",
        "listing",
        "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G",
        "corr_01HXY3NK0Z9F6S1B2C3D4",
    )
    .expect("valid");
    let json = serde_json::to_string(&log).expect("serialize");
    let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(log, back);
}

#[test]
fn serde_roundtrip_system_action_with_nones() {
    let log = AuditLog::try_new(
        Id::new(),
        None,
        "batch.cleanup",
        "notification",
        "ntf_x",
        None,
        None,
        None,
        None,
        "sys_x",
        Utc::now(),
    )
    .expect("valid");
    let json = serde_json::to_string(&log).expect("serialize");
    let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(log, back);
    assert!(back.is_system_action());
}

#[test]
fn ip_address_v4_preserved() {
    let v4 = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 42));
    let log = AuditLog::try_new(
        Id::new(),
        Some(Id::new()),
        "user.login",
        "user",
        "usr_x",
        None,
        None,
        Some(v4),
        None,
        "corr_x",
        Utc::now(),
    )
    .expect("valid");
    assert_eq!(log.ip_address, Some(v4));
}

#[test]
fn ip_address_v6_preserved() {
    let v6 = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
    let log = AuditLog::try_new(
        Id::new(),
        Some(Id::new()),
        "user.login",
        "user",
        "usr_x",
        None,
        None,
        Some(v6),
        None,
        "corr_x",
        Utc::now(),
    )
    .expect("valid");
    assert_eq!(log.ip_address, Some(v6));
}

#[test]
fn ip_address_none_preserved() {
    let log = AuditLog::try_new(
        Id::new(),
        None,
        "system.tick",
        "system",
        "tick",
        None,
        None,
        None,
        None,
        "sys_x",
        Utc::now(),
    )
    .expect("valid");
    assert!(log.ip_address.is_none());
}

#[test]
fn before_after_state_jsonb_roundtrip() {
    let before = serde_json::json!({"price": 100, "status": "draft"});
    let after = serde_json::json!({"price": 200, "status": "published", "extra": [1, 2, 3]});
    let log = AuditLog::try_new(
        Id::new(),
        Some(Id::new()),
        "listing.update",
        "listing",
        "lst_x",
        Some(before.clone()),
        Some(after.clone()),
        None,
        None,
        "corr_x",
        Utc::now(),
    )
    .expect("valid");
    let json = serde_json::to_string(&log).expect("serialize");
    let back: AuditLog = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.before_state, Some(before));
    assert_eq!(back.after_state, Some(after));
}

#[test]
fn trim_normalizes_action_resource_kind_resource_id_correlation_id() {
    let log = make_full(
        "  listing.published  ",
        "  listing  ",
        "  lst_x  ",
        "  corr_x  ",
    )
    .expect("valid");
    assert_eq!(log.action, "listing.published");
    assert_eq!(log.resource_kind, "listing");
    assert_eq!(log.resource_id, "lst_x");
    assert_eq!(log.correlation_id, "corr_x");
}

#[test]
fn whitespace_only_action_rejected_as_empty() {
    let err = make_full("    ", "listing", "lst_x", "corr_x").unwrap_err();
    assert!(matches!(err, AuditLogError::EmptyAction));
}

#[test]
fn boundary_action_exactly_100_chars_accepted() {
    let exactly = "X".repeat(100);
    let log = make_full(&exactly, "listing", "lst_x", "corr_x").expect("100 ok");
    assert_eq!(log.action.chars().count(), 100);
}

#[test]
fn boundary_user_agent_exactly_500_chars_accepted() {
    let exactly = "X".repeat(500);
    let log = AuditLog::try_new(
        Id::new(),
        Some(Id::new()),
        "user.login",
        "user",
        "usr_x",
        None,
        None,
        None,
        Some(exactly.clone()),
        "corr_x",
        Utc::now(),
    )
    .expect("500 ok");
    assert_eq!(log.user_agent.as_deref().map(str::len), Some(500));
    assert_eq!(log.user_agent, Some(exactly));
}

#[test]
fn created_at_matches_now_argument() {
    let now = Utc::now();
    let log = AuditLog::try_new(
        Id::new(),
        None,
        "system.tick",
        "system",
        "tick",
        None,
        None,
        None,
        None,
        "sys_x",
        now,
    )
    .expect("valid");
    assert_eq!(log.created_at, now);
}

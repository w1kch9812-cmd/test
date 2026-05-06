//! `Notification` 도메인 (Insights BC, RDS 동적).
//!
//! 사용자 알림. append-mostly (이벤트 발생 시 1 row).
//! `mark_read`는 멱등 — 이미 읽은 알림 재호출 시 `read_at` 보존.
//! 365일 retention — 워커가 더 오래된 row를 삭제.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod kind;
pub mod repository;

pub use kind::NotificationKind;

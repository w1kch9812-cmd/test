//! `SystemAlert` Aggregate — 시스템 알림 (severity + acknowledge / resolve).
//!
//! Spec § 5.5 `system_alert` 매핑.
//!
//! - **No OCC** — `version` 컬럼 없음.
//! - `severity` 4값 — `info` / `warning` / `error` / `critical`.
//!   `is_actionable()` = `Error|Critical`.
//! - `acknowledge(by, at)` — 1회만. 재호출 시 `AlreadyAcknowledged`.
//! - `resolve(at)` — 1회만. 재호출 시 `AlreadyResolved`. 사전 acknowledge 불필요
//!   (시스템 자동 복구로 인한 auto-resolve 가능).
//! - `metadata` JSONB — 호출자가 자유롭게 채움.

pub mod entity;
pub mod errors;
pub mod severity;

pub use entity::SystemAlert;
pub use errors::SystemAlertError;
pub use severity::SystemAlertSeverity;

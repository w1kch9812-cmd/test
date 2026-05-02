//! `ListingReport` 도메인 (Operations BC, RDS 동적).
//!
//! 사용자(혹은 익명 방문자)가 매물에 대해 신고를 접수하면 어드민이 *조사 중*
//! (`investigating`) → *확정* (`confirmed`) / *기각* (`dismissed`) 으로 처리하는
//! 워크플로우.
//!
//! - **4-status workflow**: `Open` → `Investigating` → `Confirmed` / `Dismissed`.
//! - **No OCC**: admin 신고 처리는 동시 충돌이 드물어 `version` 컬럼이 없어요.
//! - **익명 허용**: `reporter_id` 는 `NULL` 가능 (`Option<Id<UserMarker>>`).
//! - **6 reasons**: `FakeListing`, `WrongPrice`, `WrongLocation`,
//!   `InappropriateContent`, `Spam`, `Other`.
//! - **handler 메모 필수**: `mark_confirmed` / `mark_dismissed` 는 trim 후 비어있으면 거부.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod reason;
pub mod repository;
pub mod status;

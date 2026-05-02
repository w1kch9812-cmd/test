//! `Listing` Aggregate (Core BC, RDS 동적).
//!
//! 매물 도메인 — 매매/월세/전세 거래 유형 + 상태 머신
//! (`Draft` → `PendingReview` → `Active` → `{Sold, Expired}`).
//!
//! 본 task (T10): struct + `try_new_draft` (V003_01 invariant 강제).
//! T11에서 도메인 메서드 (`submit_for_review`/`approve`/`mark_sold` 등) + `Repository` trait 추가.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;

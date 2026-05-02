//! `ListingReviewQueue` (LRQ) 도메인 (Operations BC, RDS 동적).
//!
//! 매물 검토 큐 — 사용자가 등록한 매물을 어드민이 검토해 *승인* / *거부* / *변경 요청*
//! (`request_changes`) 처리하는 워크플로우.
//!
//! - **Decision-based workflow**: `decision = None` (pending) →
//!   `Some(Approve)` / `Some(Reject)` / `Some(RequestChanges)` (terminal).
//! - **12h SLA**: `submitted_at + 12h` 로 `sla_due_at` 자동 계산.
//! - **Optimistic concurrency**: `version` 컬럼으로 동시 검토 충돌 차단.
//! - **Auto check**: 룰 기반 자동 점수 (`auto_check_score`, 0-100) + 플래그
//!   (`auto_check_flags` JSONB) 를 큐 생성 시 기록.
//! - **Once-only**: 한 번 결정되면 이후 모든 결정 시도는 `AlreadyDecided` 에러.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod decision;
pub mod entity;
pub mod errors;
pub mod repository;

//! `BusinessVerificationQueue` (BVQ) 도메인 (Operations BC, RDS 동적).
//!
//! 사업자 인증 큐 — 사용자가 제출한 사업자등록증 등의 문서 (R2 keys) 를 어드민이
//! 검토해 *승인* / *거부* / *추가 자료 요청* 처리하는 워크플로우.
//!
//! - **4-status workflow**: `Pending` / `Approved` / `Rejected` / `NeedsMoreInfo`.
//! - **24h SLA**: `submitted_at + 24h` 로 `sla_due_at` 자동 계산.
//! - **Optimistic concurrency**: `version` 컬럼으로 동시 검토 충돌 차단.
//! - **Append guarantees**: `Approved` / `Rejected` 는 terminal.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod repository;
pub mod status;

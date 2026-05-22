//! `Listing` 도메인 메서드 단위 테스트 — 상태 전이 + counter.
//!
//! `entity.rs`에서 `#[path = "methods_tests.rs"] mod methods_tests;`로 포함.
//! 파일 자체가 테스트 모듈이라 별도 `mod tests {}` 래퍼 없어요.

#![allow(clippy::expect_used, clippy::unwrap_used)]
// SP6-iii: record_bookmark / release_bookmark 가 #[deprecated] 처리됨 — 단위 테스트는 FU 70 까지 보존.
#![allow(deprecated)]

#[path = "methods_tests/counters.rs"]
mod counters;
#[path = "methods_tests/fixtures.rs"]
mod fixtures;
#[path = "methods_tests/state_transitions.rs"]
mod state_transitions;
#[path = "methods_tests/update_editable_fields.rs"]
mod update_editable_fields;

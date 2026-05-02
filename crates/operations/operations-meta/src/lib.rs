//! Operations Meta BC — `FeaturedContent` + `SystemAlert` 두 Aggregate 합본.
//!
//! Spec § 5.5 `featured_content` / `system_alert` 두 테이블 매핑하는 Aggregate
//! 와 합쳐진 [`OperationsMetaRepository`] trait 를 제공해요.
//!
//! 둘 다 Operations BC 의 *meta* 데이터 (워크플로우 X) 이고 `version` OCC 컬럼이
//! 없어 단일 trait + 단순 `RepoError` (no `Conflict`) 로 묶었어요.
//!
//! ## ID prefix 주의
//!
//! - `FeaturedContent` — Spec inline 은 `fc_` (2-char) 로 적혀있지만 본 프로젝트
//!   30자 ID 불변식 (3-char prefix + `_` + 26-char ULID) 충족 위해 **`fea`** 사용.
//!   Plan 2c T17 결정. Spec FU 11 에서 reconcile 예정.
//! - `SystemAlert` — Spec inline `sal_` 와 일치.
//!
//! [`OperationsMetaRepository`]: crate::repository::OperationsMetaRepository

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod alert;
pub mod featured;
pub mod repository;

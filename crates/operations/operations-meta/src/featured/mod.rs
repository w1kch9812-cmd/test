//! `FeaturedContent` Aggregate — 홈페이지 추천/광고/스폰서/뉴스레터 노출.
//!
//! Spec § 5.5 `featured_content` 매핑.
//!
//! - **No OCC** — `version` 컬럼 없음 (admin 광고 운영 동시 충돌 드뭄).
//! - **`V003_03` invariant** — `ends_at > starts_at` (DB CHECK + Aggregate 검증).
//! - `target_kind` 3값 — `listing` / `industrial_complex` / `manufacturer`.
//! - `feature_kind` 4값 — `homepage_featured` / `search_top` / `sponsored_marker` /
//!   `newsletter`.
//! - 카운터 (`impression_count` / `click_count`) 는 saturating add — admin 레이어의
//!   동시성 race 는 비즈니스적으로 허용해요.

pub mod entity;
pub mod errors;
pub mod feature_kind;
pub mod target_kind;

pub use entity::FeaturedContent;
pub use errors::FeaturedContentError;
pub use feature_kind::FeaturedContentFeatureKind;
pub use target_kind::FeaturedContentTargetKind;

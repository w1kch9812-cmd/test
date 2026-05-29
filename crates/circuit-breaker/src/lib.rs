//! 공짱 외부 API 호출 표준 미들웨어 — Policy + Breaker + execute.
//!
//! 모든 Gongzzang-owned 외부 API 호출은 이 crate 의 [`execute`] 를 통과해
//! timeout + retry + circuit breaking 을 받아요.
//!
//! 사용 예 (Platform Core published API):
//! ```ignore
//! use circuit_breaker::{execute, Breaker, Policy};
//!
//! let breaker = Breaker::new();
//! let policy = Policy::platform_core_default();
//! let result = execute(&breaker, &policy, "platform_core.catalog", || async {
//!     reqwest::get(url).await
//! }).await?;
//! ```
//!
//! # 정책
//! - 모든 외부 API 호출은 *반드시* 이 미들웨어 통과 (FU 26: lint 차단 예정)
//! - 각 API 정책은 owning adapter 또는 service boundary의 이름 있는 상수로 표현
//! - Open 진입은 `tracing::warn!` — Sentry alert / Slack 통합은 SP7 관측성

pub mod breaker;
pub mod execute;
pub mod policy;

pub use breaker::{Breaker, CircuitState};
pub use execute::{execute, BreakerError};
pub use policy::Policy;

//! 공짱 외부 API 호출 표준 미들웨어 — Policy + Breaker + execute.
//!
//! 모든 외부 API 호출 (V-World / data.go.kr / 법제처 / Naver Maps 등) 은 이
//! crate 의 [`execute`] 를 통과해 timeout + retry + circuit breaking 을 받아요.
//!
//! 사용 예 (V-World):
//! ```ignore
//! use circuit_breaker::{execute, Breaker, Policy};
//!
//! let breaker = Breaker::new();
//! let policy = Policy::vworld_default();
//! let result = execute(&breaker, &policy, "vworld.parcel", || async {
//!     reqwest::get(url).await
//! }).await?;
//! ```
//!
//! # 정책
//! - 모든 외부 API 호출은 *반드시* 이 미들웨어 통과 (FU 26: lint 차단 예정)
//! - 각 API 정책은 [`Policy::vworld_default`] / `data_go_kr_default()` 등 이름 있는
//!   상수로 표현 — 호출 측에서 정책 의미가 즉시 명확
//! - Open 진입은 `tracing::warn!` — Sentry alert / Slack 통합은 SP7 관측성

pub mod breaker;
pub mod execute;
pub mod policy;

pub use breaker::{Breaker, CircuitState};
pub use execute::{execute, BreakerError};
pub use policy::Policy;

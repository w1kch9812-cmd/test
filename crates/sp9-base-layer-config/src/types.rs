//! SSS-grade newtypes — invalid state 를 *컴파일 시점* 에 차단.
//!
//! 본 모듈은 SP9 base layer 의 *값-객체* (value object) 들을 박제. 모든 newtype 은:
//! - **생성자에서 검증** — `new(s)` 가 invalid input 을 `Err` 로 거부, panic 0.
//! - **`Display` / `AsRef<str>`** — 호출자가 `format!` / `&str` API 양쪽에 자연 흘려보냄.
//! - **`Serialize` / `Deserialize`** — JSON manifest 에 그대로 박제 (검증 round-trip).
//! - **`Debug` / `Clone` / `Eq` / `Hash`** — collection / 로깅 친화.
//!
//! 사용 정책: ETL pipeline 의 *모든* path 가 본 newtype 들을 직접 받아야 하며 `String` /
//! `&str` 으로의 fallback 은 금지. 환경변수 / CLI 인자 / config 파일 어느 origin 이든
//! `new()` 한 번만 통과해 *internal type* 이 된 후 사용한다.

mod environment;
mod error;
mod r2_public_base;
mod srs;
#[cfg(test)]
mod tests;
mod version;

pub use environment::Environment;
pub use error::{EnvironmentParseError, TypeError};
pub use r2_public_base::R2PublicBase;
pub use srs::Srs;
pub use version::Version;

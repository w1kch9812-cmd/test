//! API drift 검출 record 도메인 — SP7-iii.
//!
//! 정부 API (data.go.kr / V-World 등) 의 nightly cron 검증 결과를
//! 우리 Postgres `api_health_check` 테이블에 영구 record.
//!
//! - [`HealthStatus`] — 6 분류 (`success` / `http_5xx` / `http_4xx` / `parse_fail` / `timeout` / `connection_fail`)
//! - [`HealthCheckRecord`] — DB row 도메인 표현
//! - [`NewHealthCheck`] — `INSERT` 용 빌더
//! - [`HealthCheckRepository`] — port trait, `crates/db` 가 `PgImpl` 제공

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod repository;
pub mod status;

pub use entity::{HealthCheckRecord, NewHealthCheck};
pub use repository::{HealthCheckRepository, RepoError};
pub use status::HealthStatus;

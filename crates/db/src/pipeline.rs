//! Postgres implementation for pipeline schedules and pipeline runs.
//!
//! This module keeps the public repository type stable while splitting the
//! storage concerns into focused private modules:
//! - `rows`: database row to domain aggregate conversion.
//! - `repository`: SQL persistence, audit log, and outbox writes.

#![allow(clippy::module_name_repetitions)]

mod repository;
mod rows;

use sqlx::PgPool;

/// `PipelineSchedule` and `PipelineRun` aggregate repository backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgPipelineRepository {
    pool: PgPool,
}

impl PgPipelineRepository {
    /// Create a repository using the shared Postgres pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

//! Data Pipeline Control BC — `PipelineSchedule` (cron) + `PipelineRun` (status machine + steps).
//!
//! Spec § 5.4 `pipeline_schedule` / `pipeline_run` 테이블 매핑하는 두 Aggregate 와
//! 합쳐진 [`PipelineRepository`] trait 를 제공해요.
//!
//! [`PipelineRepository`]: crate::repository::PipelineRepository

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod errors;
pub mod repository;
pub mod run;
pub mod schedule;
pub mod status;
pub mod trigger_kind;

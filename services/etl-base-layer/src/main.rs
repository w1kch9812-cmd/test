//! Gongzzang static vector tile ETL binary.
//!
//! The binary is intentionally thin: it initializes process-level runtime concerns, then
//! delegates subcommand behavior to focused modules.

#![forbid(unsafe_code)]
// T2 (Round 2): R2 calls are wrapped by circuit-breaker policy; direct Semaphore use here is intentional.
#![allow(clippy::disallowed_types)]

mod bronze;
mod bronze_cli;
mod cli;
mod config;
mod dtmk_prepare;
mod error;
mod gold;
mod gold_cli;
mod gold_upload;
mod handover;
mod manifest;
mod r2_upload;
mod runtime;
#[cfg(test)]
mod test_support;
mod verify_cli;

use std::process::ExitCode;

use tracing::error;

fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    let _sentry_guard = runtime::init_sentry();
    runtime::init_tracing();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            error!(error = %error, "tokio runtime build failed");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(cli::async_main())
}

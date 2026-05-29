//! Gongzzang static vector tile ETL binary.
//!
//! Platform Core owns the static vector tile artifact lifecycle. This binary is kept only
//! as a handover stub so legacy Gongzzang jobs fail closed with an ownership notice.

#![forbid(unsafe_code)]

mod cli;
mod handover;
mod runtime;

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

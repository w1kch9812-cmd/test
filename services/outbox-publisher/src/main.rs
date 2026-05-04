//! 공짱 outbox publisher daemon — `outbox_event` row 를 폴링해 `Sink` 로 발행.
//!
//! 환경변수:
//! - `DATABASE_URL` (필수) — `Postgres` 접속 문자열
//! - `OUTBOX_POLL_INTERVAL_MS` (기본 1000) — tick 주기
//! - `OUTBOX_BATCH_SIZE` (기본 100) — tick 당 fetch limit
//! - `RUST_LOG` (기본 `info`) — `tracing-subscriber` env filter
//!
//! 종료 신호 (`SIGTERM` / `Ctrl+C`) 받으면 진행 중 tick 완료 후 graceful shutdown.

#![forbid(unsafe_code)]
// `main.rs`: init failure panic은 정답이라 expect/unwrap 허용해요.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::env;
use std::sync::Arc;
use std::time::Duration;

use db::outbox::PgOutboxRepository;
use outbox_event_domain::repository::OutboxRepository;
use outbox_publisher::{tick, LoggingSink};
use sqlx::postgres::PgPoolOptions;
use tokio::signal;
use tokio::time;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let interval_ms: u64 = env::var("OUTBOX_POLL_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let batch_size: u32 = env::var("OUTBOX_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepository::new(pool));
    let sink = LoggingSink::new();

    info!(interval_ms, batch_size, "outbox publisher starting");

    let mut interval = time::interval(Duration::from_millis(interval_ms));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match tick(repo.as_ref(), &sink, batch_size).await {
                    Ok(report) if report.fetched > 0 => {
                        info!(
                            fetched = report.fetched,
                            published = report.published,
                            failed = report.failed,
                            "tick"
                        );
                    }
                    Ok(_) => {} // empty tick — silent (운영 spam 방지)
                    Err(e) => error!(error = %e, "tick failed"),
                }
            }
            () = shutdown_signal() => {
                info!("shutdown signal received — stopping");
                break;
            }
        }
    }
}

/// `SIGTERM` (Unix) / `Ctrl+C` 대기.
///
/// Windows 빌드는 `SIGTERM` 미지원 — `pending::<()>()` 로 대체해 `Ctrl+C` 만 동작.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("install ctrl-c handler");
    };
    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        _ = term => {}
    }
}

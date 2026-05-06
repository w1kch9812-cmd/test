//! Sentry / OTLP / Prometheus 관측성 init helpers (SP-Obs T5+T6).
//!
//! 모두 *env-driven*. 미설정 시 silent disabled (개발 환경 호환). Production
//! 은 SP8 `IaC` 가 환경변수 주입.

use std::env;

/// Sentry init — `SENTRY_DSN` 미설정 시 None (no-op).
///
/// `release` = `gongzzang-api@<GIT_SHA>` (build-time set, 미설정 시 cargo
/// version). `environment` = `APP_ENV` (default `dev`).
///
/// `traces_sample_rate: 0.1` — 10% 샘플링. production 트래픽 시 조절.
///
/// 반환된 `ClientInitGuard` 는 `main` 의 lifetime 동안 유지 — Drop 시 flush.
#[must_use]
#[allow(clippy::cognitive_complexity)] // env 분기 + ClientOptions builder — 쪼개면 더 모호.
pub fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = env::var("SENTRY_DSN").ok().filter(|s| !s.trim().is_empty())?;

    let release = env::var("GIT_SHA").map_or_else(
        |_| format!("gongzzang-api@{}", env!("CARGO_PKG_VERSION")),
        |sha| format!("gongzzang-api@{sha}"),
    );

    let env_label = env::var("APP_ENV").unwrap_or_else(|_| "dev".to_owned());

    let guard = sentry::init(sentry::ClientOptions {
        dsn: dsn.parse().ok(),
        release: Some(release.into()),
        environment: Some(env_label.into()),
        traces_sample_rate: 0.1,
        attach_stacktrace: true,
        ..sentry::ClientOptions::default()
    });

    if guard.is_enabled() {
        tracing::info!("sentry initialized");
        Some(guard)
    } else {
        tracing::warn!("sentry init returned disabled guard — DSN parse 실패 가능");
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    /// `SENTRY_DSN` 미설정 시 None 반환 (silent disabled).
    #[test]
    fn init_sentry_returns_none_when_dsn_missing() {
        // env 격리 위해 unique key 사용 — Rust 테스트는 thread-shared env. 실제
        // SENTRY_DSN 설정 환경에서는 본 테스트 의미 없음 — CI 보장은 별도.
        // 본 테스트는 *helper 로직* 검증 (env 없을 때 path).
        env::remove_var("SENTRY_DSN");
        assert!(init_sentry().is_none());
    }

    #[test]
    fn init_sentry_returns_none_when_dsn_empty() {
        env::set_var("SENTRY_DSN", "");
        assert!(init_sentry().is_none());
        env::remove_var("SENTRY_DSN");
    }

    #[test]
    fn init_sentry_returns_none_when_dsn_whitespace_only() {
        env::set_var("SENTRY_DSN", "   ");
        assert!(init_sentry().is_none());
        env::remove_var("SENTRY_DSN");
    }
}

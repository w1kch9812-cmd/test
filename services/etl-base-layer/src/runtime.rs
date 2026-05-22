use std::process::ExitCode;

use sp9_base_layer_config::{EnvironmentParseError, R2PublicBase};
use tracing::error;
use tracing_subscriber::EnvFilter;

use crate::config::{Config, ConfigError};

pub fn load_config_or_exit() -> Result<Config, ExitCode> {
    Config::from_env().map_err(|e| {
        log_config_error(&e);
        ExitCode::from(2)
    })
}

fn log_config_error(error: &ConfigError) {
    match error {
        ConfigError::Environment(parse_err) => log_environment_config_error(parse_err),
        ConfigError::InvalidGoldVersion { raw, detail } => log_invalid_gold_version(raw, detail),
        ConfigError::PartialR2Namespace {
            prefix,
            present,
            missing,
        } => log_partial_r2_namespace(prefix, present, missing),
    }
}

fn log_environment_config_error(error: &EnvironmentParseError) {
    error!(
        error = %error,
        "ETL_ENVIRONMENT required (ADR 0029) — set to one of: local | staging | production"
    );
}

fn log_invalid_gold_version(raw: &str, detail: &str) {
    error!(
        raw = %raw,
        detail = %detail,
        "GOLD_VERSION invalid (ADR 0035 typed err) — must match ^v[a-z0-9_-]+$"
    );
}

fn log_partial_r2_namespace(prefix: &str, present: &[String], missing: &[String]) {
    error!(
        prefix = %prefix,
        present = ?present,
        missing = ?missing,
        "R2 namespace credentials partial (ADR 0035) — atomic 4-of-4 required (credential mix 차단)"
    );
}

/// 환경변수가 set 되어 있고 trim 후 비어있지 않으면 `Some(value)`, 아니면 `None`.
/// Round 3 P1 — license / source URL / `correlation_id` 같은 *옵션 lineage* 항목용.
pub fn nonempty_env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

/// `R2_PUBLIC_URL_BASE` env → [`R2PublicBase`] 검증된 newtype.
///
/// 미설정 / 빈 문자열 / scheme 위반 / host 부재 모두 fail-fast — placeholder URL 발행 0.
/// Codex Round 6 finding #7 — production 환경에서는 `http://` 거부 (TLS 강제).
/// dev / staging 은 localhost 등 http 허용.
pub fn read_r2_public_base() -> Result<R2PublicBase, String> {
    let raw = std::env::var("R2_PUBLIC_URL_BASE")
        .map_err(|_| "R2_PUBLIC_URL_BASE env is not set".to_owned())?;
    if raw.trim().is_empty() {
        return Err("R2_PUBLIC_URL_BASE is empty".to_owned());
    }
    let base = R2PublicBase::new(raw).map_err(|e| e.to_string())?;
    if sp9_base_layer_config::Environment::is_production_from_env()
        && base.as_str().to_ascii_lowercase().starts_with("http://")
    {
        return Err(
            "R2_PUBLIC_URL_BASE must use https:// in production (ADR 0035 + finding #7)".to_owned(),
        );
    }
    Ok(base)
}

pub fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,etl_base_layer=debug"));
    // L4: prod 는 ETL_LOG_FORMAT=json (CloudWatch / Datadog 자동 파싱). dev = pretty (default).
    let json_mode = std::env::var("ETL_LOG_FORMAT").as_deref() == Ok("json");

    // sentry-tracing layer — error/warn level 의 tracing event 자동 Sentry breadcrumb 변환.
    // SENTRY_DSN 미설정 시 init_sentry 가 None 반환 → sentry::Hub 가 no-op → layer 도 무동작.
    let sentry_layer = sentry_tracing::layer().enable_span_attributes();

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(sentry_layer);
    if json_mode {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true).json())
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .init();
    }
}

/// L4 — Sentry SDK init. `SENTRY_DSN` 미설정 시 no-op (silent disabled).
/// release / environment / `git_sha` 자동 박제 → Sentry UI 의 release tracking 활성.
pub fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = std::env::var("SENTRY_DSN")
        .ok()
        .filter(|v| !v.trim().is_empty())?;
    let release = std::env::var("GIT_SHA").ok().map(Into::into);
    // ADR 0035 — `ETL_ENVIRONMENT` SSOT only. backward-compat `ETL_BUILD_ENV` 제거.
    let environment: std::borrow::Cow<'static, str> = std::env::var("ETL_ENVIRONMENT")
        .unwrap_or_else(|_| "dev".to_owned())
        .into();
    // Round 3 P1 — traces_sample_rate env-driven (이전에 0.0 hardcode → SLO 측정 불가).
    // ETL 월 1회 cron 이라 traces=1.0 도 비용 무관, 단 dev / CI smoke 는 0.0 default.
    // production workflow 가 `SENTRY_TRACES_SAMPLE_RATE=1.0` 명시 set.
    let traces_sample_rate: f32 = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release,
            environment: Some(environment),
            // 100% sampling — ETL 은 월 1회 cron 이라 비용 무관, 모든 에러 보고.
            sample_rate: 1.0,
            traces_sample_rate,
            ..Default::default()
        },
    ));

    // Round 3 P1 — correlation_id 를 Sentry global scope tag 로 박제. 모든 에러 / span
    // 이 본 ID 와 cross-reference 가능 (Sentry UI 의 search filter, log aggregator 등).
    if let Some(corr_id) =
        nonempty_env_var("CORRELATION_ID").or_else(|| nonempty_env_var("GITHUB_RUN_ID"))
    {
        sentry::configure_scope(|scope| {
            scope.set_tag("correlation_id", &corr_id);
        });
    }

    Some(guard)
}

use tracing_subscriber::EnvFilter;

fn nonempty_env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,etl_base_layer=debug"));
    let json_mode = std::env::var("ETL_LOG_FORMAT").as_deref() == Ok("json");
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

pub fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = std::env::var("SENTRY_DSN")
        .ok()
        .filter(|value| !value.trim().is_empty())?;
    let release = std::env::var("GIT_SHA").ok().map(Into::into);
    let environment: std::borrow::Cow<'static, str> = std::env::var("ETL_ENVIRONMENT")
        .unwrap_or_else(|_| "dev".to_owned())
        .into();
    let traces_sample_rate: f32 = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release,
            environment: Some(environment),
            sample_rate: 1.0,
            traces_sample_rate,
            ..Default::default()
        },
    ));

    if let Some(correlation_id) =
        nonempty_env_var("CORRELATION_ID").or_else(|| nonempty_env_var("GITHUB_RUN_ID"))
    {
        sentry::configure_scope(|scope| {
            scope.set_tag("correlation_id", &correlation_id);
        });
    }

    Some(guard)
}

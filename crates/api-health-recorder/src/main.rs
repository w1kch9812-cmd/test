//! API health recorder — SP7-iii GitHub Actions cron 후속 단계.
//!
//! 사용법:
//! ```bash
//! cargo run --bin api-health-recorder -- \
//!     --api-name data_go_kr.getBrTitleInfo \
//!     --status success \
//!     --http-code 200 \
//!     --duration-ms 1234 \
//!     --cron-run true
//! ```
//!
//! 동작:
//! 1. [`PgHealthCheckRepository::record()`] 로 DB INSERT.
//! 2. fail 인 경우:
//!    - hard-fail (`http_4xx` / `parse_fail`) → 즉시 GitHub Issue.
//!    - soft-fail (`http_5xx` / `timeout` / `connection_fail`) + 3일 연속 cron fail → Issue.
//!    - else → record only.
//! 3. success 인 경우:
//!    - 기존 open `drift` Issue (`api_name` 일치) 자동 close + 자가 복구 comment.
//!
//! 환경변수:
//! - `DATABASE_URL` (필수) — `PgPool` 연결.
//! - `GITHUB_TOKEN` (필수) — Issue 생성 / close.
//! - `GITHUB_REPOSITORY` (필수, GitHub Actions 자동 set) — `owner/repo`.

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use api_health_domain::{HealthCheckRepository, HealthStatus, NewHealthCheck};
use clap::Parser;
use db::api_health::PgHealthCheckRepository;
use octocrab::Octocrab;
use sqlx::PgPool;
use tracing::{info, warn};

/// Args (clap derive).
#[derive(Parser, Debug)]
#[command(name = "api-health-recorder")]
struct Args {
    /// API endpoint 식별자. 예: `data_go_kr.getBrTitleInfo`.
    #[arg(long)]
    api_name: String,

    /// `HealthStatus` 문자열. `success` / `http_5xx` / `http_4xx` / `parse_fail` / `timeout` / `connection_fail`.
    #[arg(long)]
    status: String,

    /// HTTP 응답 코드 (선택).
    #[arg(long)]
    http_code: Option<u16>,

    /// 마스킹된 에러 디테일 (선택).
    #[arg(long)]
    error_detail: Option<String>,

    /// `true` = scheduled cron, `false` = `workflow_dispatch`.
    ///
    /// 명시적 값 파싱 (`--cron-run true` / `--cron-run false`) — clap flag mode 회피.
    /// GitHub Actions workflow 가 `"${{ github.event_name == 'schedule' }}"` 문자열을 그대로 전달하므로
    /// `num_args = 1` 으로 value 강제.
    #[arg(long, num_args = 1, value_parser = clap::value_parser!(bool))]
    cron_run: bool,

    /// 호출 소요 시간 (ms).
    #[arg(long)]
    duration_ms: u32,
}

/// 3일 연속 cron fail 임계값.
const STREAK_THRESHOLD: u32 = 3;
/// drift Issue label.
const ISSUE_LABEL: &str = "drift";

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let args = Args::parse();
    let status = HealthStatus::from_str(&args.status)
        .with_context(|| format!("invalid --status: {}", args.status))?;

    // 1. DB record.
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL required")?;
    let pool = PgPool::connect(&database_url).await.context("connect DB")?;
    let repo = PgHealthCheckRepository::new(Arc::new(pool));

    let new = NewHealthCheck {
        api_name: &args.api_name,
        status,
        http_code: args.http_code,
        error_detail: args.error_detail.as_deref(),
        cron_run: args.cron_run,
        duration_ms: args.duration_ms,
    };
    let record = repo.record(new).await.context("record to DB")?;
    info!(
        record_id = record.id,
        api = %record.api_name,
        status = %record.status,
        "recorded to api_health_check"
    );

    // 2. GitHub Issue orchestration.
    let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN required")?;
    let repo_full = std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY required")?;
    let (owner, repo_name) = repo_full
        .split_once('/')
        .with_context(|| format!("GITHUB_REPOSITORY 형식 'owner/repo' 필요: {repo_full}"))?;

    let octo = Octocrab::builder().personal_token(token).build()?;

    let escalate = if status.is_hard_fail() {
        true
    } else if status.is_soft_fail() {
        repo.is_n_cron_runs_failed(&args.api_name, STREAK_THRESHOLD)
            .await
            .context("query streak")?
    } else {
        false
    };

    if escalate {
        let ctx = DriftIssueContext {
            owner,
            repo: repo_name,
            args: &args,
            status,
            error_detail: record.error_detail.as_deref(),
        };
        create_or_update_drift_issue(&octo, &ctx).await?;
    } else if status == HealthStatus::Success {
        recover_open_drift_issues(&octo, owner, repo_name, &args.api_name).await?;
    } else {
        info!("soft-fail without 3-day streak — record only");
    }

    Ok(())
}

/// `create_or_update_drift_issue` 호출에 필요한 컨텍스트.
struct DriftIssueContext<'a> {
    owner: &'a str,
    repo: &'a str,
    args: &'a Args,
    status: HealthStatus,
    error_detail: Option<&'a str>,
}

/// 기존 open drift Issue 가 있으면 comment append, 없으면 새 Issue 생성.
async fn create_or_update_drift_issue(octo: &Octocrab, ctx: &DriftIssueContext<'_>) -> Result<()> {
    let issues_handler = octo.issues(ctx.owner, ctx.repo);
    let title = format!("🚨 정부 API drift detected: {}", ctx.args.api_name);

    let list = issues_handler
        .list()
        .labels(&[ISSUE_LABEL.to_owned()])
        .state(octocrab::params::State::Open)
        .send()
        .await
        .context("list issues")?;

    if let Some(existing) = list.items.iter().find(|i| i.title == title) {
        append_drift_comment(&issues_handler, existing.number, ctx).await
    } else {
        create_drift_issue(&issues_handler, &title, ctx).await
    }
}

/// 이미 열린 drift Issue 에 후속 fail comment 추가.
async fn append_drift_comment(
    issues_handler: &octocrab::issues::IssueHandler<'_>,
    number: u64,
    ctx: &DriftIssueContext<'_>,
) -> Result<()> {
    let comment = format!(
        "또 fail (cron_run={}, status={}, http={:?}).\n\n```\n{}\n```",
        ctx.args.cron_run,
        ctx.status,
        ctx.args.http_code,
        ctx.error_detail.unwrap_or("(no detail)")
    );
    issues_handler
        .create_comment(number, comment)
        .await
        .context("create comment")?;
    warn!(issue = number, "appended comment to existing drift issue");
    Ok(())
}

/// 새 drift Issue 생성.
async fn create_drift_issue(
    issues_handler: &octocrab::issues::IssueHandler<'_>,
    title: &str,
    ctx: &DriftIssueContext<'_>,
) -> Result<()> {
    let body = format!(
        "## 발견 시각\n{}\n\n## 분류\n{}\n\n## API\n{}\n\n## 응답 정보\n- HTTP: {:?}\n- duration_ms: {}\n- cron_run: {}\n\n## 실패 log\n```\n{}\n```\n\n## 수동 검증\nGitHub Actions → \"api-drift-smoke-test\" → \"Run workflow\"",
        chrono::Utc::now().to_rfc3339(),
        ctx.status,
        ctx.args.api_name,
        ctx.args.http_code,
        ctx.args.duration_ms,
        ctx.args.cron_run,
        ctx.error_detail.unwrap_or("(no detail)")
    );

    let labels = vec![
        ISSUE_LABEL.to_owned(),
        format!("drift:{}", status_label_suffix(ctx.status)),
    ];

    let new_issue = issues_handler
        .create(title)
        .body(body)
        .labels(labels)
        .send()
        .await
        .context("create issue")?;
    warn!(issue = new_issue.number, "created drift issue");
    Ok(())
}

/// success 인 경우 기존 open drift Issue 를 자가 복구 comment 후 close.
async fn recover_open_drift_issues(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    api_name: &str,
) -> Result<()> {
    let issues_handler = octo.issues(owner, repo);
    let list = issues_handler
        .list()
        .labels(&[ISSUE_LABEL.to_owned()])
        .state(octocrab::params::State::Open)
        .send()
        .await
        .context("list issues")?;

    let title = format!("🚨 정부 API drift detected: {api_name}");

    for issue in list.items.iter().filter(|i| i.title == title) {
        let comment = "✅ 자가 복구 — 정부 일시 장애였음. 다음 cron 정상 응답으로 close.";
        issues_handler
            .create_comment(issue.number, comment)
            .await
            .context("create recovery comment")?;
        issues_handler
            .update(issue.number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await
            .context("close issue")?;
        info!(issue = issue.number, "closed drift issue (auto-recovered)");
    }
    Ok(())
}

/// `HealthStatus` → 짧은 label suffix.
const fn status_label_suffix(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Success => "success",
        HealthStatus::Http5xx => "5xx-server",
        HealthStatus::Http4xx => "4xx-auth",
        HealthStatus::ParseFail => "schema",
        HealthStatus::Timeout => "timeout",
        HealthStatus::ConnectionFail => "connection",
    }
}

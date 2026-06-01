# SP7-iii API Drift Monitoring - Part 03A: API Health Recorder

Parent index: [SP7-iii API Drift Monitoring - Part 03](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-03.md).
### Task 5: T5 — `crates/api-health-recorder/` (octocrab + PgImpl 재사용)

**Files:**
- Create: `crates/api-health-recorder/Cargo.toml`
- Create: `crates/api-health-recorder/src/main.rs`
- Modify: `Cargo.toml` (workspace members 추가)

#### Step 5.1: workspace Cargo.toml members 에 추가

- [ ] **Step**: `Cargo.toml` 에 추가

```toml
"crates/api-health-recorder",
```

#### Step 5.2: api-health-recorder Cargo.toml

- [ ] **Step**: `crates/api-health-recorder/Cargo.toml` 작성

```toml
[package]
name = "api-health-recorder"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[[bin]]
name = "api-health-recorder"
path = "src/main.rs"

[dependencies]
api-health-domain = { path = "../operations/api-health" }
db = { path = "../db" }
sqlx = { workspace = true, features = ["runtime-tokio", "postgres"] }
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
chrono = { workspace = true }
clap = { workspace = true, features = ["derive"] }
octocrab = "0.46"
anyhow = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true, features = ["env-filter"] }
```

**참고**: `clap`/`anyhow`/`tracing-subscriber` 가 workspace deps 에 있는지 확인. 없으면 직접 버전 명시.

#### Step 5.3: main.rs 작성

- [ ] **Step**: `crates/api-health-recorder/src/main.rs` 작성

```rust
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
//! 1. `PgHealthCheckRepository::record()` 로 DB INSERT
//! 2. fail 인 경우:
//!    - hard-fail (4xx / parse_fail) → 즉시 GitHub Issue
//!    - soft-fail (5xx / timeout / connection_fail) + 3일 연속 cron fail → Issue
//!    - else → record only
//! 3. success 인 경우:
//!    - 기존 open `drift` Issue (`api_name` 일치) 자동 close + comment
//!
//! 환경변수:
//! - `DATABASE_URL` (필수) — PgPool 연결
//! - `GITHUB_TOKEN` (필수) — Issue 생성/close
//! - `GITHUB_REPOSITORY` (필수, 자동 set in actions) — `owner/repo` 형식

#![allow(clippy::expect_used)]

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};
use api_health_domain::{
    HealthCheckRepository, HealthStatus, NewHealthCheck,
};
use clap::Parser;
use db::api_health::PgHealthCheckRepository;
use octocrab::Octocrab;
use sqlx::PgPool;
use tracing::{info, warn};

#[derive(Parser, Debug)]
#[command(name = "api-health-recorder")]
struct Args {
    /// API endpoint 식별자. 예: data_go_kr.getBrTitleInfo
    #[arg(long)]
    api_name: String,

    /// HealthStatus 문자열. success / http_5xx / http_4xx / parse_fail / timeout / connection_fail
    #[arg(long)]
    status: String,

    /// HTTP 응답 코드 (선택).
    #[arg(long)]
    http_code: Option<u16>,

    /// masked log (선택).
    #[arg(long)]
    error_detail: Option<String>,

    /// true = scheduled cron, false = workflow_dispatch.
    #[arg(long)]
    cron_run: bool,

    /// 호출 소요 시간 (ms).
    #[arg(long)]
    duration_ms: u32,
}

const STREAK_THRESHOLD: u32 = 3;
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

    // 1. DB record
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

    // 2. GitHub Issue orchestration
    let token = std::env::var("GITHUB_TOKEN").context("GITHUB_TOKEN required")?;
    let repo_full = std::env::var("GITHUB_REPOSITORY").context("GITHUB_REPOSITORY required")?;
    let (owner, repo_name) = repo_full
        .split_once('/')
        .with_context(|| format!("GITHUB_REPOSITORY 형식 'owner/repo' 필요, got: {repo_full}"))?;

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
        create_or_update_drift_issue(&octo, owner, repo_name, &args, status, &record.error_detail).await?;
    } else if status == HealthStatus::Success {
        recover_open_drift_issues(&octo, owner, repo_name, &args.api_name).await?;
    } else {
        info!("soft-fail without 3-day streak — record only");
    }

    Ok(())
}

async fn create_or_update_drift_issue(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    args: &Args,
    status: HealthStatus,
    error_detail: &Option<String>,
) -> Result<()> {
    let issues = octo.issues(owner, repo);

    // 기존 open issue 검색 (label="drift" + api_name 매치)
    let list = issues
        .list()
        .labels(&[ISSUE_LABEL.to_owned()])
        .state(octocrab::params::State::Open)
        .send()
        .await?;

    let title_match = format!("🚨 정부 API drift detected: {}", args.api_name);

    if let Some(existing) = list.items.iter().find(|i| i.title == title_match) {
        // 기존 issue → comment 추가
        let comment = format!(
            "또 fail (cron_run={}, status={}, http={:?}).\n\n```\n{}\n```",
            args.cron_run,
            status,
            args.http_code,
            error_detail.as_deref().unwrap_or("(no detail)")
        );
        issues.create_comment(existing.number, comment).await?;
        warn!(issue = existing.number, "appended comment to existing drift issue");
    } else {
        // 신규 issue
        let body = format!(
            "## 발견 시각\n{}\n\n## 분류\n{}\n\n## API\n{}\n\n## 응답 정보\n- HTTP: {:?}\n- duration_ms: {}\n- cron_run: {}\n\n## 실패 log\n```\n{}\n```\n\n## 수동 검증\nGitHub Actions → \"api-drift-smoke-test\" → \"Run workflow\"",
            chrono::Utc::now().to_rfc3339(),
            status,
            args.api_name,
            args.http_code,
            args.duration_ms,
            args.cron_run,
            error_detail.as_deref().unwrap_or("(no detail)")
        );

        let labels = vec![
            ISSUE_LABEL.to_owned(),
            format!("drift:{}", status_label_suffix(status)),
        ];

        let new_issue = issues
            .create(&title_match)
            .body(&body)
            .labels(labels)
            .send()
            .await?;
        warn!(issue = new_issue.number, "created drift issue");
    }
    Ok(())
}

async fn recover_open_drift_issues(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    api_name: &str,
) -> Result<()> {
    let issues = octo.issues(owner, repo);
    let list = issues
        .list()
        .labels(&[ISSUE_LABEL.to_owned()])
        .state(octocrab::params::State::Open)
        .send()
        .await?;

    let title_match = format!("🚨 정부 API drift detected: {api_name}");

    for issue in list.items.iter().filter(|i| i.title == title_match) {
        let comment = "✅ 자가 복구 — 정부 일시 장애였음. 다음 cron 정상 응답으로 close.".to_owned();
        issues.create_comment(issue.number, comment).await?;
        issues.update(issue.number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await?;
        info!(issue = issue.number, "closed drift issue (auto-recovered)");
    }
    Ok(())
}

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
```

**참고**: `octocrab` 0.46 의 정확한 API (issues / list / state / labels / create_comment / update) 는 plan 작성 시점에 docs.rs 확인하고 변경 가능. 위 코드는 일반적인 패턴.

#### Step 5.4: cargo check + clippy

- [ ] **Step**: 검증

```bash
cargo check -p api-health-recorder
cargo clippy -p api-health-recorder --all-targets -- -D warnings
cargo fmt --all -- --check
```

Expected: 모두 pass.

#### Step 5.5: 로컬 dry-run (선택)

- [ ] **Step**: DB 만 record 검증 (Issue API 안 호출하는 mock GH_TOKEN)

```bash
DATABASE_URL=$DATABASE_URL \
GITHUB_TOKEN=invalid_token_for_db_only_test \
GITHUB_REPOSITORY=w1kch9812-cmd/test \
cargo run --bin api-health-recorder -- \
    --api-name test.local_dry_run \
    --status success \
    --duration-ms 100 \
    --cron-run false
```

Expected: DB record 성공 + GitHub API 호출 시 `recover_open_drift_issues` 가 빈 list 반환 (token 무효지만 search 자체는 try 함). Issue 생성 분기에 안 들어가니 token error 무시 가능.

(GitHub API 가 invalid token 에 401 반환할 수 있음 → main 함수 종료 코드 1. CI 에서만 정확한 token 으로 검증.)

#### Step 5.6: T5 commit

- [ ] **Step**: commit

```bash
git add Cargo.toml crates/api-health-recorder/

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t5): add api-health-recorder Rust binary (octocrab + PgImpl)

T5 of SP7-iii:
- crates/api-health-recorder/ — workspace 신규 binary crate
  - Args (clap derive): --api-name --status --http-code --error-detail --cron-run --duration-ms
  - 1) PgHealthCheckRepository::record() 로 DB INSERT
  - 2) hard-fail (4xx/parse_fail) → 즉시 GitHub Issue
       soft-fail + 3일 연속 cron fail → Issue
       else → record only
  - 3) Success → 기존 open drift Issue 자동 close + 자가 복구 comment
- octocrab 0.46 GitHub API client (Issue create/comment/close)
- 의존성: api-health-domain + db + sqlx + tokio + clap + octocrab + anyhow + tracing
- 로컬 dry-run 검증 (DB record only)
EOF
)"
```

**사용자 체크포인트**: T5 commit 확인 + 다음 진행 여부.

---

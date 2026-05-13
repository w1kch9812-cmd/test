//! `raw-capture-sync` — fallback 디스크 → R2 sync worker (one-shot).
//!
//! ADR 0026 + Codex round 7 후속. `R2RawCapture` 가 R2 PUT 실패 시 디스크 fallback
//! 으로 적재 + `parcel_external_data.r2_object_key = "fallback::{path}"` 표기. 본
//! 워커가 그 pending row 들을 찾아 R2 로 sync + DB UPDATE.
//!
//! # 실행 패턴
//!
//! cron 외부 driver (예: `*/5 * * * * raw-capture-sync`). 한 번 실행 후 종료. 분산
//! 락 필요 없음 (UPSERT 멱등성 + 같은 파일을 두 인스턴스가 동시 처리해도 R2 PUT 은
//! 같은 키에 같은 body 라 무해).
//!
//! # 종료 코드
//!
//! - 0 = 모든 pending 처리 완료 (빈 큐 포함)
//! - 1 = 환경 / DB / 설정 에러 (재시도 의미 없음 — 운영 alert)
//! - 2 = 일부 처리 실패 (다음 사이클 재시도 가능)

#![forbid(unsafe_code)]

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use aws_credential_types::Credentials;
use aws_sdk_s3::config::{
    BehaviorVersion, Builder as S3ConfigBuilder, Region, RequestChecksumCalculation,
    ResponseChecksumValidation,
};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::{error, info, warn};

const FALLBACK_PREFIX: &str = "fallback::";
/// 한 사이클당 처리 row 상한 — DB / R2 부하 안전장치. 운영 모니터링 후 조정.
const BATCH_SIZE: i64 = 100;

#[derive(Debug)]
struct Config {
    database_url: String,
    bucket: String,
    fallback_dir: PathBuf,
    s3: S3Client,
}

impl Config {
    fn from_env() -> Result<Self, String> {
        let database_url = require_env("DATABASE_URL")?;
        let account_id = require_env("R2_ACCOUNT_ID")?;
        let access_key = require_env("R2_ACCESS_KEY")?;
        let secret_key = require_env("R2_SECRET_KEY")?;
        let bucket = require_env("R2_BUCKET")?;
        let fallback_dir = require_env("BRONZE_FALLBACK_DIR").map(PathBuf::from)?;

        let creds = Credentials::new(&access_key, &secret_key, None, None, "raw-capture-sync");
        let endpoint = format!("https://{account_id}.r2.cloudflarestorage.com");
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(3))
            .timeout_config(
                aws_config::timeout::TimeoutConfig::builder()
                    .operation_attempt_timeout(Duration::from_secs(30))
                    .build(),
            )
            .build();
        Ok(Self {
            database_url,
            bucket,
            fallback_dir,
            s3: S3Client::from_conf(s3_config),
        })
    }
}

fn require_env(name: &'static str) -> Result<String, String> {
    match env::var(name) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(format!("env {name} missing or empty")),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "config load failed");
            return ExitCode::from(1);
        }
    };

    let pool = match PgPoolOptions::new()
        .max_connections(2)
        .connect(&cfg.database_url)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            error!(error = %e, "DB connect failed");
            return ExitCode::from(1);
        }
    };

    match run_once(&cfg, &pool).await {
        Ok(stats) => {
            info!(
                total = stats.total,
                synced = stats.synced,
                failed = stats.failed,
                "raw-capture-sync cycle complete"
            );
            if stats.failed > 0 {
                ExitCode::from(2)
            } else {
                ExitCode::from(0)
            }
        }
        Err(e) => {
            error!(error = %e, "raw-capture-sync cycle aborted");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug, Default)]
struct CycleStats {
    total: usize,
    synced: usize,
    failed: usize,
}

#[allow(clippy::cognitive_complexity)] // batch loop + tracing 분기 — 분해 시 helper 의미 미미.
async fn run_once(cfg: &Config, pool: &PgPool) -> Result<CycleStats, sqlx::Error> {
    // pending fallback row 조회. r2_object_key = 'fallback::{disk_path}' 패턴.
    let pending: Vec<(String, String, String)> = sqlx::query_as(
        r"
        select pnu, source, r2_object_key
        from parcel_external_data
        where r2_object_key like 'fallback::%'
        order by fetched_at asc
        limit $1
        ",
    )
    .bind(BATCH_SIZE)
    .fetch_all(pool)
    .await?;

    let mut stats = CycleStats {
        total: pending.len(),
        ..Default::default()
    };

    if pending.is_empty() {
        info!("no pending fallback rows");
        return Ok(stats);
    }

    info!(pending = stats.total, "processing fallback rows");

    for (pnu, source, marker) in pending {
        match sync_one(cfg, pool, &pnu, &source, &marker).await {
            Ok(new_key) => {
                info!(
                    pnu = %pnu,
                    source = %source,
                    new_r2_key = %new_key,
                    "fallback synced to R2"
                );
                stats.synced += 1;
            }
            Err(e) => {
                warn!(
                    pnu = %pnu,
                    source = %source,
                    marker = %marker,
                    error = %e,
                    "sync failed — leaving for next cycle"
                );
                stats.failed += 1;
            }
        }
    }

    Ok(stats)
}

async fn sync_one(
    cfg: &Config,
    pool: &PgPool,
    pnu: &str,
    source: &str,
    marker: &str,
) -> Result<String, String> {
    // 1) marker 에서 디스크 경로 추출.
    let disk_path = marker
        .strip_prefix(FALLBACK_PREFIX)
        .ok_or_else(|| format!("marker missing '{FALLBACK_PREFIX}' prefix: {marker}"))?;
    let disk_pathbuf = PathBuf::from(disk_path);

    // 2) fallback_dir prefix 제거 → R2 키 복원.
    //    R2RawCapture::write_fallback 가 fallback_dir.join(key) 로 적재 → 역산.
    let r2_key = disk_pathbuf
        .strip_prefix(&cfg.fallback_dir)
        .map_err(|e| format!("disk path not under fallback_dir: {e}"))?
        .to_string_lossy()
        // Windows 경로 separator → forward slash (R2 키 표준).
        .replace('\\', "/");

    // 3) 파일 read.
    let body = std::fs::read(&disk_pathbuf)
        .map_err(|e| format!("read {}: {e}", disk_pathbuf.display()))?;

    // 4) R2 PUT (멱등 — 같은 키에 같은 body 두 번 PUT 해도 무해).
    cfg.s3
        .put_object()
        .bucket(&cfg.bucket)
        .key(&r2_key)
        .body(ByteStream::from(body))
        .content_type("application/json")
        .send()
        .await
        .map_err(|e| format!("R2 PUT {r2_key}: {e}"))?;

    // 5) DB UPDATE — r2_object_key 를 진짜 R2 키로 교체.
    sqlx::query(
        r"
        update parcel_external_data
        set r2_object_key = $1
        where pnu = $2 and source = $3 and r2_object_key = $4
        ",
    )
    .bind(&r2_key)
    .bind(pnu)
    .bind(source)
    .bind(marker)
    .execute(pool)
    .await
    .map_err(|e| format!("DB UPDATE: {e}"))?;

    // 6) 디스크 파일 삭제 — DB UPDATE 성공 후만. 실패 시 *재시도 시 같은 키 PUT* 멱등.
    if let Err(e) = std::fs::remove_file(&disk_pathbuf) {
        warn!(
            disk_path = %disk_pathbuf.display(),
            error = %e,
            "failed to delete fallback file after sync (cleanup best-effort)"
        );
    }

    Ok(r2_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_env_missing_returns_err() {
        // env 변경 없이 검증 — Rust 1.80+ 의 unsafe set_var 회피 (forbid(unsafe_code)).
        // 채워진 값 path 는 실 운영 환경 (binary 진입점) 에서 자동 검증.
        let result = require_env("__GONGZZANG_SYNC_TEST_NEVER_SET_DEFINITELY__");
        assert!(matches!(result, Err(error) if error.contains("missing or empty")));
    }

    #[test]
    fn fallback_prefix_constant() {
        // marker 가 prefix 로 시작하는지 검증 — 프로토콜 호환성 보호.
        assert_eq!(FALLBACK_PREFIX, "fallback::");
        let marker = format!("{FALLBACK_PREFIX}/var/lib/gongzzang/bronze/x.json");
        assert!(marker.starts_with(FALLBACK_PREFIX));
        assert_eq!(
            marker.strip_prefix(FALLBACK_PREFIX),
            Some("/var/lib/gongzzang/bronze/x.json")
        );
    }
}

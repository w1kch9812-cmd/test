//! 환경변수 → 정적 설정 매핑 (Round 5+ — ADR 0029 environment 명시 분리).
//!
//! 모든 설정은 환경변수 driven — secret 은 GitHub Actions secrets 또는 ECS task
//! environment 로 주입. **`ETL_ENVIRONMENT` 명시 필수** (미설정 시 fail-fast).
//!
//! ## Environment namespace (ADR 0029)
//!
//! 각 env 별 R2 자격이 *별도 namespace* 통과:
//! - `ETL_ENVIRONMENT=local`      → `R2_LOCAL_*`
//! - `ETL_ENVIRONMENT=staging`    → `R2_STAGING_*`
//! - `ETL_ENVIRONMENT=production` → `R2_PRODUCTION_*`
//!
//! 사고 예방 (Round 5 verify smoke 사고):
//! - 이전: `R2_*` 가 4개 다 set 시 *어떤 env 든* R2 자동 활성 → local 이 prod modify
//! - 새: `ETL_ENVIRONMENT` 명시 필요 + namespace 격리 → local 에서 `R2_PRODUCTION_*`
//!   set 돼있어도 ETL 가 *읽지 않음* (의도 격리)
//!
//! ## Backward-compat (1 sprint)
//!
//! 기존 `R2_*` (namespace 없음) 도 *fallback* 으로 허용 — *경고 로그* 출력 후 활성.
//! ADR 0030 (후속) 에서 완전 제거.

use std::env;
use std::path::PathBuf;

use sp9_base_layer_config::{Environment, EnvironmentParseError, Version};
use tracing::warn;

use crate::r2_upload::R2Config;

/// SHP/GeoJSON 다운로드 source 정의.
///
/// 공공데이터포털 SHP zip 은 분기 갱신 — 본 ETL 이 매월 1일 실행하지만, 같은 url
/// 이라도 sha256 비교로 실 변경 검출 (변경 없으면 Gold 빌드 skip).
#[derive(Debug, Clone)]
pub struct BronzeSource {
    /// 식별자 (R2 key prefix 에 사용 — `parcel`/`admin`/`industrial-complex`).
    pub id: &'static str,
    /// 다운로드 URL (공공데이터포털 / V-World / 기타).
    pub url: String,
    /// 로컬 파일명 (e.g. `parcel.shp.zip`).
    pub filename: &'static str,
}

/// ETL 설정 — ADR 0029 environment 명시 분리.
#[derive(Debug, Clone)]
pub struct Config {
    /// 실행 환경 — `ETL_ENVIRONMENT` SSOT. local / staging / production.
    /// 향후 namespace 가 활성 path 분기 + Sentry tag / lineage 박제에 활용.
    #[allow(dead_code)]
    pub environment: Environment,
    /// Bronze 산출물 저장 디렉터리 (R2 업로드 전 임시 캐시).
    /// 기본값 `./var/bronze`. Container 환경에서는 mount 된 volume.
    pub bronze_dir: PathBuf,
    /// 배치 실행 시각 라벨 (R2 prefix `<YYYY-MM>` 에 사용).
    pub batch_label: String,
    /// 다운로드할 소스들. 환경변수 미설정 시 빈 vec.
    pub sources: Vec<BronzeSource>,
    /// R2 자격 증명 + 버킷. 미설정 시 `None` — 로컬 전용 모드.
    pub r2: Option<R2Config>,
    /// Gold 버전 라벨 (newtype — `^v[a-z0-9_-]+$` 검증).
    #[allow(dead_code)]
    pub gold_version: Option<Version>,
}

/// `Config::from_env` 의 fail-fast 에러.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `ETL_ENVIRONMENT` 미설정 / invalid (ADR 0029 fail-fast).
    #[error("environment: {0}")]
    Environment(#[from] EnvironmentParseError),
}

impl Config {
    /// 환경변수에서 [`Config`] 로드. `ETL_ENVIRONMENT` 명시 필수.
    ///
    /// # Errors
    ///
    /// - `ETL_ENVIRONMENT` 미설정 / invalid → [`ConfigError::Environment`]
    pub fn from_env() -> Result<Self, ConfigError> {
        // ADR 0029 — environment 명시 필수.
        let environment = Environment::from_env_required()?;

        let bronze_dir = env::var("BRONZE_DIR")
            .unwrap_or_else(|_| "./var/bronze".to_owned())
            .into();
        let batch_label = env::var("BRONZE_BATCH_LABEL")
            .unwrap_or_else(|_| chrono::Utc::now().format("%Y-%m").to_string());

        let mut sources = Vec::new();
        if let Ok(url) = env::var("BRONZE_PARCEL_SHP_URL") {
            if !url.trim().is_empty() {
                sources.push(BronzeSource {
                    id: "parcel",
                    url,
                    filename: "parcel.shp.zip",
                });
            }
        }
        if let Ok(url) = env::var("BRONZE_ADMIN_SHP_URL") {
            if !url.trim().is_empty() {
                sources.push(BronzeSource {
                    id: "admin",
                    url,
                    filename: "admin.shp.zip",
                });
            }
        }
        if let Ok(url) = env::var("BRONZE_COMPLEX_GEOJSON_URL") {
            if !url.trim().is_empty() {
                sources.push(BronzeSource {
                    id: "industrial-complex",
                    url,
                    filename: "industrial-complex.geojson",
                });
            }
        }

        // ADR 0029 — environment 별 namespace 통과. namespace 누락 시 backward-compat 로
        // 일반 `R2_*` 시도 (1 sprint 한정, warning 후 활성).
        let r2 = build_r2_config_namespaced(environment.r2_secret_prefix())
            .or_else(|| build_r2_config_legacy(environment));

        let gold_version = env::var("GOLD_VERSION")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .map(|raw| {
                Version::new(raw.clone()).unwrap_or_else(|e| {
                    #[allow(clippy::panic)]
                    {
                        panic!("GOLD_VERSION invalid: {e} (raw={raw:?})")
                    }
                })
            });

        Ok(Self {
            environment,
            bronze_dir,
            batch_label,
            sources,
            r2,
            gold_version,
        })
    }
}

/// ADR 0029 namespace R2 config 로드 — `<prefix>R2_ACCOUNT_ID` 등 4개 모두 set 시만 활성.
///
/// 예: `prefix = "R2_PRODUCTION_"` → `R2_PRODUCTION_ACCOUNT_ID` / `R2_PRODUCTION_ACCESS_KEY`
/// / `R2_PRODUCTION_SECRET_KEY` / `R2_PRODUCTION_BUCKET`. prefix 가 없는 일반 `R2_*` 와 격리.
fn build_r2_config_namespaced(prefix: &str) -> Option<R2Config> {
    let account_id = nonempty_env(&format!("{prefix}ACCOUNT_ID"))?;
    let access_key = nonempty_env(&format!("{prefix}ACCESS_KEY"))?;
    let secret_key = nonempty_env(&format!("{prefix}SECRET_KEY"))?;
    let bucket = nonempty_env(&format!("{prefix}BUCKET"))?;
    let bronze_prefix = nonempty_env(&format!("{prefix}BRONZE_PREFIX"))
        .unwrap_or_else(|| "bronze".to_owned());
    let gold_prefix = nonempty_env(&format!("{prefix}GOLD_PREFIX"))
        .unwrap_or_else(|| "gold".to_owned());
    Some(R2Config {
        account_id,
        access_key,
        secret_key,
        bucket,
        bronze_prefix,
        gold_prefix,
    })
}

/// Backward-compat (ADR 0029, 1 sprint 한정) — 기존 `R2_*` (namespace 없음).
///
/// **Local 환경 제외**: local 에서는 본 fallback 무조건 *비활성*. 본 결정은 Round 5
/// verify smoke 의 사고 (사용자 `.env` 박제 → local 이 prod R2 modify) 의 직접 후속.
/// 사용자 박제 "trick 1개라도 거부" — local 에서 legacy 자동 활성 자체가 trick.
///
/// Staging/production 에서만 backward-compat 활성 + *경고 로그* — CI 가 새 namespace
/// 로 secret 갱신할 시간 1 sprint 보장. ADR 0030 에서 완전 제거.
fn build_r2_config_legacy(env: Environment) -> Option<R2Config> {
    // SSS-grade safety — local 은 legacy fallback 절대 활성 X.
    if matches!(env, Environment::Local) {
        // local 에서 legacy R2_* 자격이 *존재* 하더라도 활성 X. 단 사용자 진단 위해 1회 경고.
        if nonempty_env("R2_ACCOUNT_ID").is_some() {
            warn!(
                "R2_* (legacy) detected in local environment — IGNORED for safety. \
                 Use R2_LOCAL_* namespace if you intend to smoke against R2. \
                 ADR 0029 (Round 5 verify smoke 사고 후속)."
            );
        }
        return None;
    }

    let account_id = nonempty_env("R2_ACCOUNT_ID")?;
    let access_key = nonempty_env("R2_ACCESS_KEY")?;
    let secret_key = nonempty_env("R2_SECRET_KEY")?;
    let bucket = nonempty_env("R2_BUCKET")?;

    warn!(
        environment = %env,
        "R2_* (no namespace) detected — ADR 0029 backward-compat path. Migrate to {prefix}* before ADR 0030 removes this fallback.",
        prefix = env.r2_secret_prefix(),
    );

    let bronze_prefix = nonempty_env("R2_BRONZE_PREFIX").unwrap_or_else(|| "bronze".to_owned());
    let gold_prefix = nonempty_env("R2_GOLD_PREFIX").unwrap_or_else(|| "gold".to_owned());
    Some(R2Config {
        account_id,
        access_key,
        secret_key,
        bucket,
        bronze_prefix,
        gold_prefix,
    })
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use std::sync::Mutex;

    /// process-global env mutation 직렬화.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_all_r2_env() {
        for k in [
            "R2_ACCOUNT_ID",
            "R2_ACCESS_KEY",
            "R2_SECRET_KEY",
            "R2_BUCKET",
            "R2_BRONZE_PREFIX",
            "R2_GOLD_PREFIX",
            "R2_LOCAL_ACCOUNT_ID",
            "R2_LOCAL_ACCESS_KEY",
            "R2_LOCAL_SECRET_KEY",
            "R2_LOCAL_BUCKET",
            "R2_STAGING_ACCOUNT_ID",
            "R2_STAGING_ACCESS_KEY",
            "R2_STAGING_SECRET_KEY",
            "R2_STAGING_BUCKET",
            "R2_PRODUCTION_ACCOUNT_ID",
            "R2_PRODUCTION_ACCESS_KEY",
            "R2_PRODUCTION_SECRET_KEY",
            "R2_PRODUCTION_BUCKET",
            "ETL_ENVIRONMENT",
        ] {
            env::remove_var(k);
        }
    }

    /// ADR 0029 — `ETL_ENVIRONMENT` 미설정 시 fail-fast.
    #[test]
    fn fail_fast_when_environment_unset() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        let err = Config::from_env().expect_err("ETL_ENVIRONMENT unset = fail-fast");
        assert!(matches!(
            err,
            ConfigError::Environment(EnvironmentParseError::Unset)
        ));
    }

    /// ADR 0029 — `ETL_ENVIRONMENT=foo` 같이 invalid 값도 fail-fast.
    #[test]
    fn fail_fast_when_environment_invalid() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "qa");
        let err = Config::from_env().expect_err("invalid env = fail-fast");
        assert!(matches!(
            err,
            ConfigError::Environment(EnvironmentParseError::Invalid(_))
        ));
        clear_all_r2_env();
    }

    /// ADR 0029 — namespace 격리: local 에서 `R2_PRODUCTION_*` 만 set 시 R2 비활성.
    /// 이게 본 ADR 의 *가장 중요한* 회귀 invariant — Round 5 사고 재발 방지.
    #[test]
    fn local_env_ignores_production_credentials() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "local");
        // production credential 만 set — local 이 *읽으면 안 됨*.
        env::set_var("R2_PRODUCTION_ACCOUNT_ID", "prod-account");
        env::set_var("R2_PRODUCTION_ACCESS_KEY", "prod-key");
        env::set_var("R2_PRODUCTION_SECRET_KEY", "prod-secret");
        env::set_var("R2_PRODUCTION_BUCKET", "prod-bucket");

        let cfg = Config::from_env().expect("load");
        assert_eq!(cfg.environment, Environment::Local);
        assert!(
            cfg.r2.is_none(),
            "local env must not auto-activate production R2 — namespace 격리 위반"
        );
        clear_all_r2_env();
    }

    /// ADR 0029 — namespace 매칭: production env + `R2_PRODUCTION_*` set 시 활성.
    #[test]
    fn production_env_uses_production_namespace() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "production");
        env::set_var("R2_PRODUCTION_ACCOUNT_ID", "p-a");
        env::set_var("R2_PRODUCTION_ACCESS_KEY", "p-k");
        env::set_var("R2_PRODUCTION_SECRET_KEY", "p-s");
        env::set_var("R2_PRODUCTION_BUCKET", "p-b");

        let cfg = Config::from_env().expect("load");
        let r2 = cfg.r2.expect("R2 activated");
        assert_eq!(r2.account_id, "p-a");
        assert_eq!(r2.bucket, "p-b");
        clear_all_r2_env();
    }

    /// ADR 0029 — backward-compat: namespace 없는 `R2_*` 도 1 sprint 한정 허용
    /// (warning 후 활성). ADR 0030 에서 제거 예정.
    #[test]
    fn legacy_r2_fallback_activates_with_warning() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "staging");
        env::set_var("R2_ACCOUNT_ID", "legacy-a");
        env::set_var("R2_ACCESS_KEY", "legacy-k");
        env::set_var("R2_SECRET_KEY", "legacy-s");
        env::set_var("R2_BUCKET", "legacy-b");

        let cfg = Config::from_env().expect("load");
        let r2 = cfg.r2.expect("legacy fallback activates");
        assert_eq!(r2.account_id, "legacy-a");
        clear_all_r2_env();
    }

    /// ADR 0029 (Round 5 사고 후속) — local 에서 legacy `R2_*` 도 *절대* 활성 안 됨.
    /// 사용자 `.env` 박제로 인한 사고 재발 차단. 본 invariant 가 ADR 0029 의 핵심.
    #[test]
    fn local_env_ignores_even_legacy_r2_credentials() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "local");
        // legacy R2_* 4개 모두 set (사용자 `.env` 박제 시나리오).
        env::set_var("R2_ACCOUNT_ID", "leak-account");
        env::set_var("R2_ACCESS_KEY", "leak-key");
        env::set_var("R2_SECRET_KEY", "leak-secret");
        env::set_var("R2_BUCKET", "leak-bucket");

        let cfg = Config::from_env().expect("load");
        assert!(
            cfg.r2.is_none(),
            "local env must IGNORE legacy R2_* — Round 5 verify smoke 사고 재발 차단"
        );
        clear_all_r2_env();
    }

    /// ADR 0029 — partial namespace credential 도 None (4개 모두 필요).
    #[test]
    fn partial_namespace_returns_none() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "staging");
        env::set_var("R2_STAGING_ACCOUNT_ID", "x");
        env::set_var("R2_STAGING_ACCESS_KEY", "y");
        // SECRET_KEY / BUCKET 누락.
        let cfg = Config::from_env().expect("load");
        assert!(cfg.r2.is_none(), "partial namespace = None");
        clear_all_r2_env();
    }
}

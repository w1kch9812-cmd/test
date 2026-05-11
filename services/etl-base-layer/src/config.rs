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
    /// `GOLD_VERSION` env 가 invalid 형식 — ADR 0030 (이전 panic path 제거).
    #[error("GOLD_VERSION invalid: {detail} (raw={raw:?})")]
    InvalidGoldVersion {
        /// 원본 env 값.
        raw: String,
        /// newtype 검증 에러 메시지.
        detail: String,
    },
    /// 부분 R2 namespace credential — ADR 0030 (credential mix 차단 fail-fast).
    /// 4개 (`ACCOUNT_ID` / `ACCESS_KEY` / `SECRET_KEY` / `BUCKET`) 중 일부만 set.
    #[error(
        "partial R2 namespace credentials at prefix {prefix:?} — set: {present:?}, missing: {missing:?}. \
         ADR 0030: namespace credential 은 atomic (4개 모두 set 또는 0개) — partial = credential mix 위험."
    )]
    PartialR2Namespace {
        /// 사용한 prefix (e.g. `R2_PRODUCTION_`).
        prefix: String,
        /// set 된 suffix 들.
        present: Vec<String>,
        /// 누락된 suffix 들.
        missing: Vec<String>,
    },
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

        // ADR 0030 — environment 별 namespace 통과. legacy `R2_*` fallback *완전 제거*
        // (ADR 0029 의 1-sprint backward-compat 자체가 trick — credential mix 위험).
        // partial namespace = typed fail-fast (Python atomic loader 와 동일 정책).
        let r2 = build_r2_config_strict(environment.r2_secret_prefix())?;

        let gold_version = match env::var("GOLD_VERSION")
            .ok()
            .filter(|v| !v.trim().is_empty())
        {
            None => None,
            Some(raw) => match Version::new(raw.clone()) {
                Ok(v) => Some(v),
                Err(e) => {
                    // ADR 0030 — panic 제거. typed ConfigError 로 호출자에게 전파.
                    return Err(ConfigError::InvalidGoldVersion {
                        raw,
                        detail: e.to_string(),
                    });
                }
            },
        };

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

/// ADR 0030 — namespace R2 config strict atomic loader.
///
/// 4개 자격 (`ACCOUNT_ID` / `ACCESS_KEY` / `SECRET_KEY` / `BUCKET`) 이 *모두 같은
/// source* (namespace) 에서 박제. partial namespace = `PartialR2Namespace` fail-fast
/// (credential mix 차단). legacy `R2_*` fallback **완전 제거** — ADR 0030.
///
/// 반환:
/// - namespace 4개 모두 set → `Ok(Some(R2Config))`
/// - namespace 4개 모두 unset → `Ok(None)` (R2 비활성, local-only mode)
/// - namespace 부분 set → `Err(ConfigError::PartialR2Namespace)` (mix 차단)
fn build_r2_config_strict(prefix: &str) -> Result<Option<R2Config>, ConfigError> {
    const SUFFIXES: [&str; 4] = ["ACCOUNT_ID", "ACCESS_KEY", "SECRET_KEY", "BUCKET"];
    let values: Vec<(&str, Option<String>)> = SUFFIXES
        .iter()
        .map(|s| (*s, nonempty_env(&format!("{prefix}{s}"))))
        .collect();

    let present: Vec<String> = values
        .iter()
        .filter_map(|(s, v)| v.as_ref().map(|_| (*s).to_owned()))
        .collect();
    let missing: Vec<String> = values
        .iter()
        .filter_map(|(s, v)| if v.is_none() { Some((*s).to_owned()) } else { None })
        .collect();

    // 4개 모두 unset → local-only mode (정상 path).
    if present.is_empty() {
        return Ok(None);
    }
    // 부분 set → typed fail-fast (credential mix 차단).
    if !missing.is_empty() {
        return Err(ConfigError::PartialR2Namespace {
            prefix: prefix.to_owned(),
            present,
            missing,
        });
    }

    // 4개 모두 set → atomic R2Config. `present.is_empty() == false` + `missing.is_empty()`
    // 이미 검증됐으니 unwrap 안전 (값 무결성).
    let mut iter = values.into_iter();
    #[allow(clippy::expect_used)]
    let account_id = iter
        .next()
        .and_then(|(_, v)| v)
        .expect("4-of-4 set invariant");
    #[allow(clippy::expect_used)]
    let access_key = iter
        .next()
        .and_then(|(_, v)| v)
        .expect("4-of-4 set invariant");
    #[allow(clippy::expect_used)]
    let secret_key = iter
        .next()
        .and_then(|(_, v)| v)
        .expect("4-of-4 set invariant");
    #[allow(clippy::expect_used)]
    let bucket = iter
        .next()
        .and_then(|(_, v)| v)
        .expect("4-of-4 set invariant");

    // bronze_prefix / gold_prefix 는 R2 자격과 무관 (bucket key prefix). namespace
    // 우선 + global fallback (`R2_BRONZE_PREFIX` 같은 non-secret config).
    let bronze_prefix = nonempty_env(&format!("{prefix}BRONZE_PREFIX"))
        .or_else(|| nonempty_env("R2_BRONZE_PREFIX"))
        .unwrap_or_else(|| "bronze".to_owned());
    let gold_prefix = nonempty_env(&format!("{prefix}GOLD_PREFIX"))
        .or_else(|| nonempty_env("R2_GOLD_PREFIX"))
        .unwrap_or_else(|| "gold".to_owned());
    Ok(Some(R2Config {
        account_id,
        access_key,
        secret_key,
        bucket,
        bronze_prefix,
        gold_prefix,
    }))
}

fn nonempty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.trim().is_empty())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;
    use crate::test_support::GLOBAL_ENV_LOCK as ENV_LOCK;

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

    /// ADR 0030 — legacy `R2_*` fallback **완전 제거**. 어떤 env 에서도 활성 X.
    /// 이전 (ADR 0029 1-sprint backward-compat) path 가 *근본이 아닌 표면 합의* 였음.
    #[test]
    fn legacy_r2_no_longer_activates_anywhere() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "staging");
        env::set_var("R2_ACCOUNT_ID", "legacy-a");
        env::set_var("R2_ACCESS_KEY", "legacy-k");
        env::set_var("R2_SECRET_KEY", "legacy-s");
        env::set_var("R2_BUCKET", "legacy-b");
        let cfg = Config::from_env().expect("load");
        assert!(
            cfg.r2.is_none(),
            "legacy R2_* must NOT activate (ADR 0030 strict path)"
        );
        clear_all_r2_env();
    }

    /// ADR 0030 — local 도 legacy 도 namespace 도 unset = 정상 path (local-only mode).
    #[test]
    fn local_env_with_no_r2_credentials_is_local_only_mode() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "local");
        let cfg = Config::from_env().expect("load");
        assert!(cfg.r2.is_none(), "local + no credentials = local-only");
        clear_all_r2_env();
    }

    /// ADR 0030 — partial namespace credential = typed `PartialR2Namespace` fail-fast.
    /// 이전 (ADR 0029) 의 partial → None 자체가 trick — credential mix 위험 path.
    #[test]
    fn partial_namespace_fails_fast() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "staging");
        env::set_var("R2_STAGING_ACCOUNT_ID", "x");
        env::set_var("R2_STAGING_ACCESS_KEY", "y");
        // SECRET_KEY / BUCKET 누락.
        let err = Config::from_env()
            .expect_err("partial namespace must fail-fast (ADR 0030)");
        match err {
            ConfigError::PartialR2Namespace { prefix, present, missing } => {
                assert_eq!(prefix, "R2_STAGING_");
                assert!(present.contains(&"ACCOUNT_ID".to_owned()));
                assert!(missing.contains(&"SECRET_KEY".to_owned()));
                assert!(missing.contains(&"BUCKET".to_owned()));
            }
            other => panic!("expected PartialR2Namespace, got {other:?}"),
        }
        clear_all_r2_env();
    }

    /// ADR 0030 — invalid `GOLD_VERSION` 가 panic 안 함 (typed err 로 propagate).
    #[test]
    fn invalid_gold_version_returns_typed_error() {
        let _g = ENV_LOCK.lock().expect("lock");
        clear_all_r2_env();
        env::set_var("ETL_ENVIRONMENT", "local");
        env::set_var("GOLD_VERSION", "V3"); // invalid — Version 은 lowercase 'v' prefix 만.
        let err = Config::from_env()
            .expect_err("invalid GOLD_VERSION must return typed err, not panic");
        assert!(
            matches!(err, ConfigError::InvalidGoldVersion { ref raw, .. } if raw == "V3"),
            "expected InvalidGoldVersion variant, got {err:?}"
        );
        env::remove_var("GOLD_VERSION");
        clear_all_r2_env();
    }
}

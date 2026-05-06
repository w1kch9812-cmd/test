//! 환경변수 → 정적 설정 매핑.
//!
//! 모든 설정은 환경변수 driven — secret 은 GitHub Actions secrets 또는 ECS task
//! environment 로 주입. 미설정 시 [`ConfigError::Missing`] 으로 fail-fast.
//!
//! 단계별 변수:
//! - **Bronze (T3a)**: `BRONZE_DIR` / `BRONZE_BATCH_LABEL` / `BRONZE_*_URL`
//! - **R2 업로드 (T3b.1)**: `R2_ACCOUNT_ID` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` /
//!   `R2_BUCKET` (+ optional prefix overrides)
//! - **Gold (T3b.2)**: `GOLD_VERSION` (활성 버전 라벨)

use std::env;
use std::path::PathBuf;

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

/// ETL 설정.
#[derive(Debug, Clone)]
pub struct Config {
    /// Bronze 산출물 저장 디렉터리 (R2 업로드 전 임시 캐시).
    /// 기본값 `./var/bronze`. Container 환경에서는 mount 된 volume.
    pub bronze_dir: PathBuf,
    /// 배치 실행 시각 라벨 (R2 prefix `<YYYY-MM>` 에 사용).
    /// `BRONZE_BATCH_LABEL` 미설정 시 `chrono::Utc::now().format("%Y-%m")` 폴백.
    pub batch_label: String,
    /// 다운로드할 소스들. 환경변수 미설정 시 빈 vec — 호출자가 별도 source 등록 가능.
    pub sources: Vec<BronzeSource>,
    /// R2 자격 증명 + 버킷. 미설정 시 `None` — 로컬 전용 모드 (T3a 동작 유지).
    pub r2: Option<R2Config>,
    /// Gold 버전 라벨 (예: `v3`). T3b.2 에서 `PMTiles` 빌드 prefix 로 사용.
    /// 미설정 시 `batch_label` 기반 폴백 (`v<YYYY-MM>` 형태는 빌드 시점에 결정).
    /// T3b.1 에서는 환경변수만 캡처 — 소비는 T3b.2 의 gold pipeline.
    #[allow(dead_code)]
    pub gold_version: Option<String>,
}

impl Config {
    /// 환경변수에서 [`Config`] 로드.
    ///
    /// **R2 동작 모드**:
    /// - `R2_ACCOUNT_ID` 가 비어있으면 → `r2 = None` (로컬 전용, T3a 호환).
    /// - 4 개 변수 (`R2_ACCOUNT_ID`/`R2_ACCESS_KEY`/`R2_SECRET_KEY`/`R2_BUCKET`)
    ///   가 모두 설정되어야 `Some(R2Config)`. 일부만 설정하면 *partial 위험* —
    ///   [`Config::from_env`] 가 그래도 `None` 반환 (로컬 전용 모드 fallback).
    ///   이 폴백은 의도적: 로컬 dev 가 실수로 `R2_BUCKET` 만 설정해도 비밀 유출 X.
    ///
    /// 변수:
    /// - `BRONZE_DIR` (선택, default `./var/bronze`)
    /// - `BRONZE_BATCH_LABEL` (선택, default 현재 UTC `%Y-%m`)
    /// - `BRONZE_PARCEL_SHP_URL` / `BRONZE_ADMIN_SHP_URL` / `BRONZE_COMPLEX_GEOJSON_URL` (선택)
    /// - `R2_ACCOUNT_ID` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` / `R2_BUCKET` (4 개 모두 → R2 활성)
    /// - `R2_BRONZE_PREFIX` (선택, default `bronze`)
    /// - `R2_GOLD_PREFIX` (선택, default `gold`)
    /// - `GOLD_VERSION` (선택)
    #[must_use]
    pub fn from_env() -> Self {
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

        let r2 = build_r2_config();
        let gold_version = env::var("GOLD_VERSION")
            .ok()
            .filter(|v| !v.trim().is_empty());

        Self {
            bronze_dir,
            batch_label,
            sources,
            r2,
            gold_version,
        }
    }
}

/// `R2_*` 환경변수 4 개가 *모두* 설정된 경우에만 `Some(R2Config)`. 부분 설정 = `None`.
fn build_r2_config() -> Option<R2Config> {
    let account_id = nonempty_env("R2_ACCOUNT_ID")?;
    let access_key = nonempty_env("R2_ACCESS_KEY")?;
    let secret_key = nonempty_env("R2_SECRET_KEY")?;
    let bucket = nonempty_env("R2_BUCKET")?;
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

    /// `R2_*` 가 하나라도 비어있으면 `None`.
    /// 환경변수 mutation 은 process-global → 같은 변수를 만지는 다른 테스트와
    /// 충돌 가능. 본 ETL crate 는 R2_* 환경변수 테스트가 이 한 곳뿐이라 무관.
    #[test]
    fn r2_partial_env_returns_none() {
        let saved: Vec<(&str, Option<String>)> = [
            "R2_ACCOUNT_ID",
            "R2_ACCESS_KEY",
            "R2_SECRET_KEY",
            "R2_BUCKET",
            "R2_BRONZE_PREFIX",
            "R2_GOLD_PREFIX",
        ]
        .iter()
        .map(|k| (*k, env::var(k).ok()))
        .collect();

        for (k, _) in &saved {
            env::remove_var(k);
        }

        // 부분만 설정.
        env::set_var("R2_ACCOUNT_ID", "x");
        env::set_var("R2_ACCESS_KEY", "y");

        assert!(
            build_r2_config().is_none(),
            "partial R2_* env should produce None"
        );

        // 모두 설정 → Some.
        env::set_var("R2_SECRET_KEY", "z");
        env::set_var("R2_BUCKET", "b");
        let cfg = build_r2_config().expect("all four set");
        assert_eq!(cfg.account_id, "x");
        assert_eq!(cfg.bucket, "b");
        assert_eq!(cfg.bronze_prefix, "bronze");
        assert_eq!(cfg.gold_prefix, "gold");

        // 복원.
        for (k, v) in saved {
            match v {
                Some(val) => env::set_var(k, val),
                None => env::remove_var(k),
            }
        }
    }
}

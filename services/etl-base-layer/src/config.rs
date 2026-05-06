//! 환경변수 → 정적 설정 매핑.
//!
//! 모든 설정은 환경변수 driven — secret 은 GitHub Actions secrets 또는 ECS task
//! environment 로 주입. 미설정 시 [`ConfigError::Missing`] 으로 fail-fast.
//!
//! Bronze 단계만 사용하는 변수와 Gold 단계 (T3b) 변수가 함께 정의되어 있음.
//! T3a 는 Bronze 만 활성 — Gold 변수는 [`Config::from_env`] 가 아직 검증 안 함.

use std::env;
use std::path::PathBuf;

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
}

impl Config {
    /// 환경변수에서 [`Config`] 로드. T3a 단계는 모든 변수가 optional fallback —
    /// 실패 케이스 없음. T3b 의 R2 secret 추가 시점부터 fallible 해질 예정.
    ///
    /// 변수:
    /// - `BRONZE_DIR` (선택, default `./var/bronze`)
    /// - `BRONZE_BATCH_LABEL` (선택, default 현재 UTC `%Y-%m`)
    /// - `BRONZE_PARCEL_SHP_URL` (선택)
    /// - `BRONZE_ADMIN_SHP_URL` (선택)
    /// - `BRONZE_COMPLEX_GEOJSON_URL` (선택)
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

        Self {
            bronze_dir,
            batch_label,
            sources,
        }
    }
}

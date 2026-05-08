//! ETL pipeline typed error — `Box<dyn std::error::Error>` 누출 제거.
//!
//! `main.rs` 의 3 개 헬퍼 함수(`prepare_dtmk_inputs` / `run_verify` /
//! `upload_gold_to_r2`) 가 공유하는 상위 error enum.
//!
//! `Box<dyn Error>` → 구체 enum 으로:
//! - 컴파일러가 모든 variant 확인 (exhaustive match).
//! - Sentry / tracing 이 source chain 을 정확히 박제.
//! - 테스트에서 `matches!()` 로 분기 검증 가능.

use thiserror::Error;

use crate::bronze::dtmk::DtmkError;
use crate::gold::build::BuildError;
use crate::gold::promote::PromoteError;
use crate::gold::shp_to_geojson::Ogr2OgrError;
use crate::gold::verify::VerifyError;
use crate::r2_upload::UploadError;

/// dtmk 준비 단계 에러 (`prepare_dtmk_inputs`).
#[derive(Debug, Error)]
pub enum PrepareError {
    /// R2 자격 미설정.
    #[error("R2 credentials not configured — set R2_ACCOUNT_ID/R2_ACCESS_KEY/R2_SECRET_KEY/R2_BUCKET")]
    R2NotConfigured,
    /// R2 list/download.
    #[error("dtmk R2: {0}")]
    Dtmk(#[from] DtmkError),
    /// ogr2ogr 사전 체크 실패.
    #[error("ogr2ogr not available: {0}")]
    Ogr2OgrUnavailable(String),
    /// ogr2ogr 변환 실패.
    #[error("ogr2ogr: {0}")]
    Ogr2Ogr(#[from] Ogr2OgrError),
    /// I/O.
    #[error("io at {path}: {source}")]
    Io {
        /// 대상 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// `tokio::task::JoinError`.
    #[error("task join: {0}")]
    Join(#[from] tokio::task::JoinError),
    /// semaphore acquire 실패 (`tokio::sync::AcquireError` — 항상 `Closed`).
    #[error("semaphore closed (tokio runtime shutdown during semaphore acquire)")]
    SemaphoreClosed,
    /// ogr2ogr spawn 작업 내부에서 String 에러 반환.
    #[error("shp conversion: {0}")]
    ShpConversion(String),
}

impl From<tokio::sync::AcquireError> for PrepareError {
    fn from(_: tokio::sync::AcquireError) -> Self {
        Self::SemaphoreClosed
    }
}

/// Gold verify 단계 에러 (`run_verify`).
#[derive(Debug, Error)]
pub enum VerifyStepError {
    /// `PMTiles` 검증 실패.
    #[error("verify: {0}")]
    Verify(#[from] VerifyError),
}

/// R2 업로드 단계 에러 (`upload_gold_to_r2`).
#[derive(Debug, Error)]
pub enum UploadStepError {
    /// `R2_PUBLIC_URL_BASE` env 미설정 (P0.2 fail-fast).
    #[error(
        "R2_PUBLIC_URL_BASE env is not set — refusing to emit placeholder URL.          Set R2_PUBLIC_URL_BASE=https://r2.example.com in environment."
    )]
    PublicUrlMissing,
    /// R2 `PutObject` / `ListObjects`.
    #[error("r2 upload: {0}")]
    R2(#[from] UploadError),
    /// sha256 계산 (I/O).
    #[error("sha256: {0}")]
    Sha256(#[from] VerifyError),
    /// promote staging spec 쓰기.
    #[error("staging spec: {0}")]
    Promote(#[from] PromoteError),
    /// Build 단계 실패 (재사용 경로에서).
    #[error("build: {0}")]
    Build(#[from] BuildError),
}

/// `R2_PUBLIC_URL_BASE` 미설정 — placeholder URL 발행 금지 (P0.2 fail-fast).
#[derive(Debug, Error)]
#[error(
    "R2_PUBLIC_URL_BASE env is not set — refusing to emit placeholder URL. \
     Set R2_PUBLIC_URL_BASE=https://r2.example.com in environment."
)]
pub struct R2PublicUrlMissing;

//! Cloudflare R2 업로드 — `aws-sdk-s3` (S3-호환) wrapper.
//!
//! R2 는 S3-호환 API 를 노출하므로 `aws-sdk-s3` 가 그대로 동작. 단, 엔드포인트는
//! `https://<account_id>.r2.cloudflarestorage.com` 형식 → [`R2Config::endpoint_url`].
//!
//! 책임:
//! - 파일 업로드 (`put_object_file`) — Bronze SHP archive / Gold `PMTiles`
//! - JSON 업로드 (`put_object_json`) — manifest / index 파일
//!
//! ## Circuit Breaker (T2 / Round 2)
//!
//! 모든 R2 호출 (`put_object_file` / `put_object_json` / `put_directory` / `list_objects`
//! / `try_get_object_bytes` / `download_to_file`) 은 [`circuit_breaker::execute`] 를 통과 —
//! [`Policy::r2_default`] 정책 (timeout 8s, max 1 retry, open after 5 fail in 10s, 60s cooldown).
//! `Breaker` 는 [`R2Uploader`] 안에 박제되어 모든 호출이 *동일* 상태 공유 — 시스템적
//! 장애 시 batch upload 가 즉시 중단되어 stream 의 나머지 PUT 도 빠르게 실패.

mod config;
mod error;
mod list_ops;
mod put_ops;
#[cfg(test)]
mod tests;
mod uploader;

pub use config::R2Config;
pub use error::UploadError;
#[allow(unused_imports)]
pub use uploader::DirectoryUploadResult;
pub use uploader::{R2Uploader, RemoteObject};

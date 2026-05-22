use thiserror::Error;

use crate::r2_upload::UploadError;

#[derive(Debug, Error)]
pub enum PromoteError {
    /// R2 API.
    #[error("r2: {0}")]
    R2(#[from] UploadError),
    /// JSON.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// 특정 layer 의 lineage 가 staging 에 없음 (build 미완 / 사용자 누락).
    #[cfg(test)]
    #[error("missing staging lineage for layer {layer} (key {key})")]
    MissingLineage {
        /// 누락 layer.
        layer: String,
        /// 기대 R2 key.
        key: String,
    },
    /// 특정 layer 의 flat tile 이 R2 에 0 개 — silent drop / partial PUT 의심.
    #[cfg(test)]
    #[error("no flat tiles found in {prefix} for layer {layer}")]
    NoFlatTiles {
        /// 누락 layer.
        layer: String,
        /// 검사한 prefix.
        prefix: String,
    },
    /// 이전 manifest 의 `current_version` 이 [`Version`] 형식 위반 (R2 외부 변조 / 구버전 manifest).
    #[cfg(test)]
    #[error("invalid previous_version in manifest: {raw:?} ({detail})")]
    InvalidPreviousVersion {
        /// manifest 에서 읽힌 원본 문자열.
        raw: String,
        /// [`sp9_base_layer_config::TypeError`] 의 사람-가독 메시지.
        detail: String,
    },
    /// HTTP 통신 (Cloudflare CDN purge).
    #[cfg(test)]
    #[error("cdn purge http: {0}")]
    Http(#[from] reqwest::Error),
    /// CDN purge 가 non-2xx 응답.
    #[cfg(test)]
    #[error("cdn purge failed status={status} body={body} body_read_error={body_read_error:?}")]
    CdnPurge {
        /// HTTP status.
        status: u16,
        /// 응답 body 처음 1024 바이트.
        body: String,
        /// Round 4 #6 — body read 자체 실패 시 그 에러 박제 (이전엔 `unwrap_or_default()`
        /// 로 silent loss). 진단 trail 보존.
        body_read_error: Option<String>,
    },
    /// Round 4 #5 — production env 에서 CDN purge config (`CLOUDFLARE_API_TOKEN` /
    /// `CLOUDFLARE_ZONE_ID` / `R2_PUBLIC_URL_BASE`) 가 누락. dev / staging 은 silent
    /// skip 허용, production 은 fail-fast (manifest 가 stale CDN 으로 publish 되는
    /// silent partial 차단).
    #[cfg(test)]
    #[error("CDN purge config missing in production: {missing} (set CLOUDFLARE_API_TOKEN / CLOUDFLARE_ZONE_ID / R2_PUBLIC_URL_BASE or override ETL_ENVIRONMENT)")]
    CdnPurgeMissingConfig {
        /// 어느 env 가 누락됐는지.
        missing: String,
    },
    /// Round 5 P1 — `cleanup_manifest_backups(keep=0)` 실수 차단.
    #[cfg(test)]
    #[error("cleanup keep must be >= 1 (refusing to delete entire backup chain)")]
    InvalidCleanupKeep,
    /// Round 5 (final) — cleanup 중 일부 backup delete 실패. 이전엔 warn 후 `Ok(())` →
    /// silent partial. 새 path: 진행은 계속 (다른 backup 도 시도) 후 typed Err 박제.
    /// workflow 가 본 에러로 exit 1 + Sentry alert.
    #[cfg(test)]
    #[error("partial cleanup: {deleted}/{attempted} succeeded, {} failed: {failures:?}", failures.len())]
    PartialCleanup {
        /// 삭제 시도한 총 개수.
        attempted: usize,
        /// 성공 개수.
        deleted: usize,
        /// 실패한 (key, error 메시지) 쌍.
        failures: Vec<(String, String)>,
    },
}

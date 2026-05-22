use circuit_breaker::BreakerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UploadError {
    /// 로컬 파일 읽기 실패.
    #[error("read file {path} failed: {source}")]
    ReadFile {
        /// 대상 파일 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// 로컬 파일 쓰기 실패 (download path).
    #[error("write file {path} failed: {source}")]
    WriteFile {
        /// 대상 파일 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// S3 `PutObject` API 실패 (네트워크 / 권한 / 4xx / 5xx).
    ///
    /// `aws_sdk_s3::error::SdkError` 는 generic 에 `Box<dyn StdError>` 가 아니어서
    /// `#[source]` 로 직접 wrap 하기 까다로움 → `DisplayErrorContext` 로 string 화.
    /// 디버깅 시에는 `RUST_LOG=aws_smithy_http=debug` 로 raw 응답 확인.
    #[error("put_object {key} failed: {detail}")]
    PutObject {
        /// 대상 객체 key.
        key: String,
        /// 원인 stringify (`DisplayErrorContext`).
        detail: String,
    },
    /// S3 `GetObject` API 실패.
    #[error("get_object {key} failed: {detail}")]
    GetObject {
        /// 대상 객체 key.
        key: String,
        /// 원인 stringify.
        detail: String,
    },
    /// S3 `ListObjectsV2` API 실패.
    #[error("list_objects {prefix} failed: {detail}")]
    ListObjects {
        /// 대상 prefix.
        prefix: String,
        /// 원인 stringify.
        detail: String,
    },
    /// `GetObject` body stream 읽기 실패.
    #[error("body stream {key} failed: {detail}")]
    BodyStream {
        /// 대상 객체 key.
        key: String,
        /// 원인 stringify.
        detail: String,
    },
    /// JSON 직렬화 실패.
    #[error("json serialize failed: {0}")]
    JsonSerialize(#[from] serde_json::Error),
    /// `put_directory` 의 `concurrency` 인자가 0 — `buffer_unordered(0)` 은 stream 정지.
    /// 호출자 정책 위반이라 컴파일 단계 차단보다 runtime fail-fast 선택.
    #[error("put_directory concurrency must be ≥ 1, got 0")]
    InvalidConcurrency,
    /// `WalkDir` 가 디렉터리 traversal 중 I/O 에러 (권한 / broken symlink / readdir fail).
    /// 이전 path 가 `filter_map(Result::ok)` 로 silent drop 하던 trick 제거.
    #[error("walk dir {root} failed: {detail}")]
    WalkDir {
        /// traversal 시작 root.
        root: String,
        /// `walkdir::Error` 의 사람-가독 메시지 (path + os error).
        detail: String,
    },
    /// `WalkDir` 가 발견한 파일을 `local_root` 의 *상대* 경로로 변환 못 함 (drive 차이 등).
    /// 이전 path 가 `unwrap_or(&abs)` 로 절대경로 키를 silent 생성하던 trick 제거.
    #[error("strip_prefix failed for {path} (root: {root})")]
    StripPrefix {
        /// 문제의 절대 경로.
        path: String,
        /// traversal root.
        root: String,
    },
    /// Circuit breaker 차단 / max-retries exceeded / timeout.
    #[error("breaker [{op}]: {detail}")]
    Breaker {
        /// 호출 op 이름 (e.g. `r2.put_object_file`).
        op: &'static str,
        /// `BreakerError::{Open|Timeout|MaxRetriesExceeded|Inner}` 의 사람-가독.
        detail: String,
    },
}

/// `BreakerError<UploadError>` → `UploadError` 변환 helper.
pub(super) fn breaker_to_upload(op: &'static str, e: BreakerError<UploadError>) -> UploadError {
    match e {
        // inner error 가 R2 SDK 호출 자체의 실패면 그 카테고리 그대로 노출 (Put/Get/List).
        BreakerError::Inner(inner) => inner,
        // 그 외 (Open / Timeout / MaxRetriesExceeded) 는 breaker variant 로 박제.
        other => UploadError::Breaker {
            op,
            detail: other.to_string(),
        },
    }
}

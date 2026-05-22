use std::path::Path;
use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use circuit_breaker::execute as breaker_execute;
use tracing::{info, instrument};

use super::error::{breaker_to_upload, UploadError};
use super::uploader::{R2Uploader, RemoteObject};

impl R2Uploader {
    /// `DeleteObject` — Round 5 P1 (ADR 0028 + runbook § 6).
    ///
    /// `manifest_backup_cleanup` 이 호출. breaker wrap 통과. idempotent —
    /// 같은 key 가 이미 없어도 `DeleteObject` 는 200 OK (S3 spec).
    ///
    /// # Errors
    ///
    /// `DeleteObject` API 실패 / circuit open / timeout.
    #[cfg(test)]
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn delete_object(&self, key: &str) -> Result<(), UploadError> {
        breaker_execute(&self.breaker, &self.policy, "r2.delete_object", || async {
            self.client
                .delete_object()
                .bucket(&self.config.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| UploadError::PutObject {
                    key: key.to_owned(),
                    detail: format!(
                        "delete_object: {}",
                        aws_sdk_s3::error::DisplayErrorContext(&e)
                    ),
                })?;
            Ok::<(), UploadError>(())
        })
        .await
        .map_err(|e| breaker_to_upload("r2.delete_object", e))?;
        Ok(())
    }

    /// `ListObjectsV2` paginated — `prefix` 하위 모든 객체 메타 반환.
    ///
    /// R2 의 `ListObjectsV2` 는 default 1000 객체/page → continuation token 으로 loop.
    /// 273 시군구 SHP zip 가정 시 1 page 면 충분하지만 안전하게 pagination 구현.
    /// T2 — 각 page 마다 breaker 통과 (long pagination 의 systemic fail 차단).
    ///
    /// # Errors
    ///
    /// `ListObjectsV2` API 실패 / circuit open / max-retries / timeout.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, prefix = %prefix))]
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<RemoteObject>, UploadError> {
        let mut all = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let token = continuation.clone();
            let resp = breaker_execute(&self.breaker, &self.policy, "r2.list_objects", || async {
                let mut req = self
                    .client
                    .list_objects_v2()
                    .bucket(&self.config.bucket)
                    .prefix(prefix);
                if let Some(t) = token.as_deref() {
                    req = req.continuation_token(t);
                }
                req.send().await.map_err(|e| UploadError::ListObjects {
                    prefix: prefix.to_owned(),
                    detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                })
            })
            .await
            .map_err(|e| breaker_to_upload("r2.list_objects", e))?;
            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    all.push(RemoteObject {
                        key: key.to_owned(),
                        size: u64::try_from(obj.size().unwrap_or(0)).unwrap_or(0),
                        etag: obj.e_tag().map(str::to_owned),
                    });
                }
            }
            if resp.is_truncated().unwrap_or(false) {
                continuation = resp.next_continuation_token().map(str::to_owned);
                if continuation.is_none() {
                    break;
                }
            } else {
                break;
            }
        }
        info!(count = all.len(), "list_objects complete");
        Ok(all)
    }

    /// `GetObject` → 로컬 파일 stream 저장. 메모리 적재 X (대용량 SHP zip 가정).
    ///
    /// 부모 디렉터리는 자동 생성. 출력 파일은 *덮어쓰기* (`fs::File::create`).
    /// idempotent skip 은 호출자가 사전 size 비교로 처리 (본 메서드는 항상 다운).
    /// T2 — `GetObject` 호출만 breaker wrap (body stream 은 connection 후 단일 흐름).
    ///
    /// # Errors
    ///
    /// `GetObject` API 실패 / body stream 실패 / 디스크 I/O 실패 / circuit open / timeout.
    #[instrument(
        skip(self, dest),
        fields(bucket = %self.config.bucket, key = %key, dest = %dest.display()),
    )]
    pub async fn download_to_file(&self, key: &str, dest: &Path) -> Result<u64, UploadError> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|source| UploadError::WriteFile {
                    path: parent.display().to_string(),
                    source,
                })?;
        }

        let resp = breaker_execute(
            &self.breaker,
            &self.policy,
            "r2.get_object_initiate",
            || async {
                self.client
                    .get_object()
                    .bucket(&self.config.bucket)
                    .key(key)
                    .send()
                    .await
                    .map_err(|e| UploadError::GetObject {
                        key: key.to_owned(),
                        detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                    })
            },
        )
        .await
        .map_err(|e| breaker_to_upload("r2.get_object_initiate", e))?;

        let mut body = resp.body;
        let mut file =
            tokio::fs::File::create(dest)
                .await
                .map_err(|source| UploadError::WriteFile {
                    path: dest.display().to_string(),
                    source,
                })?;
        let mut total: u64 = 0;
        use tokio::io::AsyncWriteExt;
        // ByteStream impl Stream → futures_util::StreamExt::next.
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| UploadError::BodyStream {
                key: key.to_owned(),
                detail: format!("{e}"),
            })?;
            file.write_all(&chunk)
                .await
                .map_err(|source| UploadError::WriteFile {
                    path: dest.display().to_string(),
                    source,
                })?;
            total += chunk.len() as u64;
        }
        file.flush()
            .await
            .map_err(|source| UploadError::WriteFile {
                path: dest.display().to_string(),
                source,
            })?;
        info!(bytes = total, "download complete");
        Ok(total)
    }

    /// `GetObject` of a *possibly-missing* object — `NoSuchKey` → `Ok(None)`.
    ///
    /// ## 왜 `get_object_bytes` 와 분리했나
    ///
    /// `get_object_bytes` 는 `NoSuchKey` 도 `UploadError::GetObject` 로 전파 → breaker 의
    /// `record_failure` 가 호출됨. 이건 manifest *first publish* 같은 *expected miss*
    /// path 에 치명적: (a) `NoSuchKey` 가 `MaxRetriesExceeded` 로 wrap → typed match 가
    /// `UploadError::GetObject` 를 못 잡음 → first publish 가 영구 실패. (b) 반복되는
    /// expected miss 가 circuit open 트리거 → 후속 정상 GET 도 차단.
    ///
    /// 본 메서드는 `NoSuchKey` 를 closure *안에서* `Ok(None)` 으로 흡수 — breaker 입장에서는
    /// 성공이라 failure window 누적 0. 다른 모든 에러 (네트워크 / 5xx / 권한) 는 그대로
    /// 전파해서 정상 breaker 로직 유지.
    ///
    /// ## 사용처
    ///
    /// - promote 단계의 `gold/manifest.json` fetch (first publish 시 None — 정상 path).
    /// - promote 의 staging spec fetch (None → typed `MissingLineage` 에러로 매핑).
    ///
    /// # Errors
    ///
    /// `GetObject` API / body stream 실패 (`NoSuchKey` 제외) / circuit open / timeout.
    #[cfg(test)]
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn try_get_object_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, UploadError> {
        let resp = breaker_execute(
            &self.breaker,
            &self.policy,
            "r2.try_get_object_bytes",
            || async {
                match self
                    .client
                    .get_object()
                    .bucket(&self.config.bucket)
                    .key(key)
                    .send()
                    .await
                {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => {
                        // NoSuchKey 는 expected miss — breaker 입장에서는 성공으로 처리.
                        if let Some(svc_err) = e.as_service_error() {
                            if svc_err.is_no_such_key() {
                                return Ok(None);
                            }
                        }
                        Err(UploadError::GetObject {
                            key: key.to_owned(),
                            detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                        })
                    }
                }
            },
        )
        .await
        .map_err(|e| breaker_to_upload("r2.try_get_object_bytes", e))?;

        let Some(resp) = resp else {
            return Ok(None);
        };

        let mut body = resp.body;
        let mut buf = Vec::new();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| UploadError::BodyStream {
                key: key.to_owned(),
                detail: format!("{e}"),
            })?;
            buf.extend_from_slice(&chunk);
        }
        Ok(Some(buf))
    }

    /// JSON pretty-encoded 객체 업로드. `content_type=application/json`.
    ///
    /// manifest / index 류에 사용 — 작은 (~10KB) 페이로드 가정.
    /// T2 — circuit breaker wrap.
    ///
    /// `T: Sync` 는 `#[instrument]` 가 만드는 future 가 multi-thread runtime
    /// 에서도 안전하게 await 되도록 강제 (clippy `future_not_send`).
    ///
    /// # Errors
    ///
    /// JSON 직렬화 실패 / `PutObject` 실패 / circuit open / max-retries / timeout.
    #[instrument(skip(self, value), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn put_object_json<T: serde::Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        cache_control: &str,
    ) -> Result<(), UploadError> {
        let json = serde_json::to_vec_pretty(value)?;
        let bytes_len = json.len();

        info!(
            r2_op = "PutObject",
            r2_bucket = %self.config.bucket,
            r2_key = %key,
            bytes = bytes_len,
            "uploading json → R2"
        );

        // breaker 의 retry 가 새 future 를 매번 만들기 때문에 body 도 매 호출마다 fresh.
        // `Arc<Vec<u8>>` 으로 share — clone 비용 감소.
        let json_arc = Arc::new(json);
        breaker_execute(
            &self.breaker,
            &self.policy,
            "r2.put_object_json",
            || async {
                let body = ByteStream::from(json_arc.as_ref().clone());
                self.client
                    .put_object()
                    .bucket(&self.config.bucket)
                    .key(key)
                    .body(body)
                    .content_type("application/json")
                    .cache_control(cache_control)
                    .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::Aes256)
                    .send()
                    .await
                    .map_err(|e| UploadError::PutObject {
                        key: key.to_owned(),
                        detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                    })?;
                Ok::<(), UploadError>(())
            },
        )
        .await
        .map_err(|e| breaker_to_upload("r2.put_object_json", e))?;

        info!("json uploaded");
        Ok(())
    }
}

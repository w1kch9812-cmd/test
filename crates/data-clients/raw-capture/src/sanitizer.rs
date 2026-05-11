//! PIPA Allowlist-based JSON sanitizer.
//!
//! `RawSanitizer` trait + `SanitizedRaw` 결과 struct. 외부 API raw 응답에서
//! allowlist 외 필드를 폐기하여 PIPA 최소수집 원칙을 컴파일 시점에 강제해요.

use serde_json::Value;

/// Sanitization 결과 — 정제된 JSON + 감사 메타데이터.
#[derive(Debug, Clone)]
pub struct SanitizedRaw {
    /// 정제된 JSON. allowlist 통과 필드만 보존.
    pub value: Value,
    /// 폐기된 path 개수 (drift detection 신호).
    pub dropped_count: usize,
    /// allowlist 정의의 SHA-256 hash (spec §5.4).
    pub schema_hash: String,
    /// AllowlistSanitizer 버전 (schema 변경 시 증가).
    pub sanitizer_version: u32,
}

/// allowlist 기반 raw JSON 정제 인터페이스.
pub trait RawSanitizer: Send + Sync {
    /// `source_id` 별 allowlist 로 raw JSON 을 정제해요.
    /// 비허용 경로는 폐기되고 `dropped_count` 에 누적돼요.
    fn sanitize(&self, raw: &Value) -> SanitizedRaw;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_raw_construct() {
        let s = SanitizedRaw {
            value: serde_json::json!({}),
            dropped_count: 0,
            schema_hash: String::new(),
            sanitizer_version: 1,
        };
        assert_eq!(s.dropped_count, 0);
        assert_eq!(s.sanitizer_version, 1);
    }
}

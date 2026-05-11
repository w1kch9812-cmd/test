//! PIPA Allowlist-based JSON sanitizer.
//!
//! `RawSanitizer` trait + `SanitizedRaw` 결과 struct. 외부 API raw 응답에서
//! allowlist 외 필드를 폐기하여 PIPA 최소수집 원칙을 컴파일 시점에 강제해요.

use serde_json::Value;
use sha2::{Digest, Sha256};

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

/// allowlist 정의의 SHA-256 hash. drift detection 의 input 이에요.
///
/// Spec §5.4 (`design.md`) 의사 코드:
///   `schema_hash = SHA-256(source_id || ":" || sanitizer_version || ":" || sorted_retained_json_paths.join(","))`
///
/// Rust 인자 매핑: `source_id` ↔ `source`, `sorted_retained_json_paths` ↔ `paths`
/// (함수 내부에서 정렬). 출력은 64-char hex digest.
#[must_use]
pub fn compute_schema_hash(source: &str, sanitizer_version: u32, paths: &[String]) -> String {
    let mut sorted_retained_json_paths = paths.to_vec();
    sorted_retained_json_paths.sort_unstable();
    let input = format!(
        "{}:{}:{}",
        source,
        sanitizer_version,
        sorted_retained_json_paths.join(",")
    );
    let digest = Sha256::digest(input.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON path-based default-deny sanitizer.
///
/// 허용된 `allowed_paths` 외의 모든 필드를 폐기해요. path 는 JSON pointer 형식
/// (`/response/header/resultCode`) + `*` wildcard (`/items/*/id`).
pub struct AllowlistSanitizer {
    source: String,
    allowed_paths: Vec<String>,
    sanitizer_version: u32,
    schema_hash: String,
}

impl AllowlistSanitizer {
    /// 직접 allowlist 를 지정하여 sanitizer 인스턴스화. test/manual use 용.
    /// production 은 `for_source` factory (T2) 사용.
    #[must_use]
    pub fn new(source: String, allowed_paths: Vec<String>, sanitizer_version: u32) -> Self {
        let schema_hash = compute_schema_hash(&source, sanitizer_version, &allowed_paths);
        Self {
            source,
            allowed_paths,
            sanitizer_version,
            schema_hash,
        }
    }

    /// source ID (예: "vworld_parcel").
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// allowed JSON paths.
    #[must_use]
    pub fn allowed_paths(&self) -> &[String] {
        &self.allowed_paths
    }

    /// sanitizer version (schema 변경 시 증가).
    #[must_use]
    pub const fn sanitizer_version(&self) -> u32 {
        self.sanitizer_version
    }

    /// SHA-256 schema hash of allowlist (drift detection).
    #[must_use]
    pub fn schema_hash(&self) -> &str {
        &self.schema_hash
    }
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

    #[test]
    fn schema_hash_deterministic() {
        let h1 = compute_schema_hash(
            "vworld_parcel",
            1,
            &["pnu".to_string(), "geometry".to_string()],
        );
        let h2 = compute_schema_hash(
            "vworld_parcel",
            1,
            &["geometry".to_string(), "pnu".to_string()],
        );
        assert_eq!(h1, h2, "path order must not affect hash");
        assert_eq!(h1.len(), 64, "SHA-256 hex digest is 64 chars");
    }

    #[test]
    fn schema_hash_version_sensitive() {
        let h1 = compute_schema_hash("vworld_parcel", 1, &["pnu".to_string()]);
        let h2 = compute_schema_hash("vworld_parcel", 2, &["pnu".to_string()]);
        assert_ne!(h1, h2, "sanitizer_version 변경 시 hash 도 변경");
    }

    #[test]
    fn schema_hash_source_sensitive() {
        let h1 = compute_schema_hash("vworld_parcel", 1, &["pnu".to_string()]);
        let h2 = compute_schema_hash("data_go_kr_building", 1, &["pnu".to_string()]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn allowlist_sanitizer_constructs_with_paths() {
        let san = AllowlistSanitizer::new(
            "test_source".to_string(),
            vec!["/a".to_string(), "/b".to_string()],
            1,
        );
        assert_eq!(san.source(), "test_source");
        assert_eq!(san.allowed_paths().len(), 2);
        assert_eq!(san.sanitizer_version(), 1);
        assert_eq!(san.schema_hash().len(), 64);
    }
}

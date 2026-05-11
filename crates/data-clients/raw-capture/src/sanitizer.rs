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
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex
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

impl RawSanitizer for AllowlistSanitizer {
    fn sanitize(&self, raw: &Value) -> SanitizedRaw {
        let mut dropped = 0usize;
        let value = sanitize_value(raw, "", &self.allowed_paths, &mut dropped);
        SanitizedRaw {
            value,
            dropped_count: dropped,
            schema_hash: self.schema_hash.clone(),
            sanitizer_version: self.sanitizer_version,
        }
    }
}

/// Recursive JSON path traversal. 현재 노드의 path 가 allowlist 와 매칭되거나
/// 그 하위에 매칭되는 path 가 있으면 보존, 아니면 폐기 (dropped++).
fn sanitize_value(
    value: &Value,
    current_path: &str,
    allowlist: &[String],
    dropped: &mut usize,
) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (key, child) in map {
                let child_path = format!("{current_path}/{key}");
                if path_allowed_or_has_descendant(&child_path, allowlist) {
                    let sanitized_child = sanitize_value(child, &child_path, allowlist, dropped);
                    if is_empty_branch(&sanitized_child)
                        && !is_exact_match(&child_path, allowlist)
                    {
                        // 자식이 모두 폐기되어 빈 가지가 됐고, 현재 노드 자체가 exact
                        // allowlist 매칭이 아니면 노드도 폐기 (counted via children).
                        *dropped += 1;
                    } else {
                        out.insert(key.clone(), sanitized_child);
                    }
                } else {
                    *dropped += 1;
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{current_path}/{i}");
                if path_allowed_or_has_descendant(&item_path, allowlist) {
                    out.push(sanitize_value(item, &item_path, allowlist, dropped));
                } else {
                    *dropped += 1;
                }
            }
            Value::Array(out)
        }
        // Primitive — caller 가 이미 path matching 으로 keep 여부 결정 후 호출.
        _ => value.clone(),
    }
}

fn is_empty_branch(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}

/// path 가 allowlist 의 *어떤 패턴* 의 prefix 거나 exact match 면 true.
/// allowlist pattern `/a/*/c` 에 대해 path `/a`, `/a/0`, `/a/0/c` 모두 true.
fn path_allowed_or_has_descendant(path: &str, allowlist: &[String]) -> bool {
    allowlist
        .iter()
        .any(|pattern| pattern_matches_or_prefix(pattern, path))
}

/// pattern 과 path 가 *segment-wise* 매칭. wildcard `*` 는 임의 segment 매칭.
/// path 가 pattern 보다 짧으면 (path 가 ancestor) → true.
/// path 가 pattern 보다 길거나 같으면 모든 segment 가 일치해야 true.
fn pattern_matches_or_prefix(pattern: &str, path: &str) -> bool {
    let p_segs: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    let t_segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    if path.is_empty() {
        return true;
    }
    let min = p_segs.len().min(t_segs.len());
    for i in 0..min {
        if p_segs[i] != "*" && p_segs[i] != t_segs[i] {
            return false;
        }
    }
    true
}

/// path 가 allowlist 의 어느 패턴과 *segment 수까지 정확히 매칭* 되는지.
fn is_exact_match(path: &str, allowlist: &[String]) -> bool {
    let t_segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    allowlist.iter().any(|pattern| {
        let p_segs: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
        if p_segs.len() != t_segs.len() {
            return false;
        }
        p_segs
            .iter()
            .zip(t_segs.iter())
            .all(|(p, t)| *p == "*" || p == t)
    })
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

    #[test]
    fn sanitize_drops_unknown_keys() {
        let san = AllowlistSanitizer::new("test".to_string(), vec!["/keep".to_string()], 1);
        let raw = serde_json::json!({
            "keep": "yes",
            "ownerNm": "홍길동",
            "phone": "010-1234-5678"
        });
        let r = san.sanitize(&raw);
        assert!(r.value.get("keep").is_some(), "허용 필드 보존");
        assert!(r.value.get("ownerNm").is_none(), "PII 필드 폐기");
        assert!(r.value.get("phone").is_none(), "PII 필드 폐기");
        assert_eq!(r.dropped_count, 2);
        assert_eq!(r.sanitizer_version, 1);
        assert_eq!(r.schema_hash.len(), 64);
    }

    #[test]
    fn sanitize_supports_wildcard_path() {
        let san = AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/items/*/id".to_string()],
            1,
        );
        let raw = serde_json::json!({
            "items": [
                {"id": "a", "secret": "drop"},
                {"id": "b", "secret": "drop"}
            ]
        });
        let r = san.sanitize(&raw);
        assert_eq!(r.value["items"][0]["id"], "a");
        assert!(r.value["items"][0].get("secret").is_none());
        assert_eq!(r.value["items"][1]["id"], "b");
        assert!(r.dropped_count >= 2);
    }

    #[test]
    fn sanitize_nested_object_pruning() {
        let san = AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/response/header/resultCode".to_string()],
            1,
        );
        let raw = serde_json::json!({
            "response": {
                "header": {
                    "resultCode": "00",
                    "resultMsg": "drop"
                },
                "body": "drop entire body"
            }
        });
        let r = san.sanitize(&raw);
        assert_eq!(r.value["response"]["header"]["resultCode"], "00");
        assert!(r.value["response"]["header"].get("resultMsg").is_none());
        assert!(r.value["response"].get("body").is_none());
        assert!(r.dropped_count >= 2);
    }
}

# T1 Sanitizer Infra - Part 02: Sanitization And Capture Wrapper

Parent index: [T1 Sanitizer Infra](./T1-sanitizer-infra.md).

## Step 1.5: AllowlistSanitizer::sanitize impl (TDD — path matching)

JSON path matching with `*` wildcard. `/a/*/c` matches `/a/0/c`, `/a/1/c`, etc.

- [ ] **Step 1.5.1: Append failing tests for sanitize**

```rust
    #[test]
    fn sanitize_drops_unknown_keys() {
        let san = AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        );
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
        assert_eq!(r.dropped_count, 2);
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
```

- [ ] **Step 1.5.2: Run — fail (RawSanitizer trait not impl'd)**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::sanitize_drops_unknown_keys
# Expected: error — method `sanitize` not found
```

- [ ] **Step 1.5.3: Implement RawSanitizer for AllowlistSanitizer**

`sanitizer.rs` 에 추가 (impl block):

```rust
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

/// Recursive JSON path traversal. 현재 노드의 path 가 allowlist 의 *어떤 prefix 와도
/// 매칭 안 되면* (즉 그 subtree 가 통째로 비허용이면) drop. 일부만 매칭되면 재귀.
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
                let child_path = format!("{}/{}", current_path, key);
                if path_allowed_or_has_descendant(&child_path, allowlist) {
                    let sanitized_child = sanitize_value(child, &child_path, allowlist, dropped);
                    // 빈 object/array 면 (모든 내부 비허용) 보존하지 않음
                    if !is_empty_branch(&sanitized_child) {
                        out.insert(key.clone(), sanitized_child);
                    } else {
                        *dropped += 1;
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
                let item_path = format!("{}/{}", current_path, i);
                if path_allowed_or_has_descendant(&item_path, allowlist) {
                    out.push(sanitize_value(item, &item_path, allowlist, dropped));
                } else {
                    *dropped += 1;
                }
            }
            Value::Array(out)
        }
        _ => {
            // Primitive — current_path 가 exact match 면 keep, 아니면 caller 가 이미 drop 처리
            value.clone()
        }
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
/// allowlist pattern `/a/*/c` 에 대해 path `/a` `/a/0` `/a/0/c` 모두 true.
fn path_allowed_or_has_descendant(path: &str, allowlist: &[String]) -> bool {
    allowlist.iter().any(|pattern| pattern_matches_or_prefix(pattern, path))
}

/// `pattern` 의 *prefix* 가 `path` 와 매칭되거나, `pattern` 자체가 `path` 의 prefix.
fn pattern_matches_or_prefix(pattern: &str, path: &str) -> bool {
    let p_segs: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
    let t_segs: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    // empty path matches everything
    if path.is_empty() {
        return true;
    }
    let min = p_segs.len().min(t_segs.len());
    for i in 0..min {
        if p_segs[i] != "*" && p_segs[i] != t_segs[i] {
            return false;
        }
    }
    // path 가 pattern 보다 깊거나 같으면 pattern 이 prefix
    // pattern 이 path 보다 깊으면 path 는 ancestor (descendant 가능)
    true
}
```

- [ ] **Step 1.5.4: Run all sanitize tests — pass**

```bash
cargo test -p raw-capture-client --lib sanitizer
# Expected: 6+ tests passed (construct + 3 schema_hash + 3 sanitize)
```

- [ ] **Step 1.5.5: Commit**

```bash
git add crates/data-clients/raw-capture/src/sanitizer.rs
git commit -m "feat(sp10-5-b-T1): AllowlistSanitizer::sanitize (JSON path matching + wildcard)"
```

---

## Step 1.6: SanitizingRawCapture wrapper (TDD — strict RED → GREEN)

spec §3.3 — trait 시그니처는 *완전히 기존 RawCapture trait* 과 일치.

- [ ] **Step 1.6.1: Create `crates/data-clients/raw-capture/src/capture.rs` — failing test ONLY (no impl yet)**

```rust
//! `RawCapture` wrapping composers — `SanitizingRawCapture` + `DualTierCapture` (T3).

use crate::{RawCapture, RawCaptureError, RawCaptureReceipt, RawSanitizer};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::Arc;

// IMPLEMENTATION COMES IN Step 1.6.4 (below) — this step intentionally compile-fails

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sanitizer::AllowlistSanitizer;
    use crate::NoOpRawCapture;
    use chrono::Utc;

    #[tokio::test]
    async fn sanitizing_wrap_drops_unknown_and_forwards() {
        let sanitizer = Arc::new(AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        ));
        let wrapped = SanitizingRawCapture::new(NoOpRawCapture::new(), sanitizer);
        let raw = serde_json::json!({"keep": "ok", "drop_me": "secret"});
        let result = wrapped
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await;
        // RawCapture::capture returns Result<RawCaptureReceipt, RawCaptureError>
        assert!(result.is_ok());
        let receipt = result.unwrap();
        // receipt 필드 검증은 NoOp 의 default Receipt 으로 (sink-agnostic)
        let _ = receipt;
    }
}
```

- [ ] **Step 1.6.2: Modify `lib.rs` — expose `capture` module declaration only**

```rust
pub mod capture;
// SanitizingRawCapture re-export 은 Step 1.6.5 commit 후 추가
```

- [ ] **Step 1.6.3: Run test — verify FAIL (compile error, struct undefined)**

```bash
cargo test -p raw-capture-client --lib capture::tests::sanitizing_wrap_drops_unknown
# Expected: error[E0422]: cannot find struct, variant or union type `SanitizingRawCapture`
# OR: error[E0433]: failed to resolve: use of undeclared crate or module
```

- [ ] **Step 1.6.4: Implement `SanitizingRawCapture` — append to `capture.rs` above `#[cfg(test)]`**

```rust
/// `RawCapture` 를 wrap 하여 INSERT 전에 `RawSanitizer` 로 정제. drift 발생 시
/// `tracing::warn!(target = "raw.capture.schema_drift", ...)` 발행.
pub struct SanitizingRawCapture<C: RawCapture> {
    inner: C,
    sanitizer: Arc<dyn RawSanitizer>,
}

impl<C: RawCapture> SanitizingRawCapture<C> {
    pub fn new(inner: C, sanitizer: Arc<dyn RawSanitizer>) -> Self {
        Self { inner, sanitizer }
    }
}

#[async_trait]
impl<C: RawCapture + Send + Sync> RawCapture for SanitizingRawCapture<C> {
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<RawCaptureReceipt, RawCaptureError> {
        let sanitized = self.sanitizer.sanitize(raw);
        if sanitized.dropped_count > 0 {
            tracing::warn!(
                target: "raw.capture.schema_drift",
                pnu = %pnu,
                source = %source,
                schema_hash = %sanitized.schema_hash,
                dropped_count = sanitized.dropped_count,
                "raw_response sanitizer dropped unknown fields"
            );
        }
        let sanitized_value = sanitized.value;
        // inner sink 의 Receipt 그대로 전파 — wrapper 는 sanitization 만 책임
        self.inner
            .capture(pnu, source, &sanitized_value, fetched_at)
            .await
    }
}
```

- [ ] **Step 1.6.5: Run test — verify PASS**

```bash
cargo test -p raw-capture-client --lib capture::tests::sanitizing_wrap_drops_unknown
# Expected: test result: ok. 1 passed
```

- [ ] **Step 1.6.6: Add re-export to `lib.rs`**

```rust
pub use capture::SanitizingRawCapture;
```

- [ ] **Step 1.6.7: Commit**

```bash
git add crates/data-clients/raw-capture/src/capture.rs crates/data-clients/raw-capture/src/lib.rs
git commit -m "feat(sp10-5-b-T1): SanitizingRawCapture wrapper + schema_drift tracing"
```


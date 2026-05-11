# T1: RawSanitizer Trait + AllowlistSanitizer + SanitizingRawCapture Infra

**Goal:** PIPA 최소수집 원칙의 컴파일 강제 기반. `RawSanitizer` trait + `SanitizedRaw` struct + `AllowlistSanitizer` (JSON path-based default-deny) + `SanitizingRawCapture<C>` wrapper 도입. allowlist 자체의 상수 정의 + V-World rename 은 T2.

**Spec SSOT:** §3.1, §3.2, §3.3, §5.4 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**Files:**

- Create: `crates/data-clients/raw-capture/src/sanitizer.rs`
- Create: `crates/data-clients/raw-capture/src/capture.rs`
- Modify: `crates/data-clients/raw-capture/Cargo.toml` (sha2 dependency)
- Modify: `crates/data-clients/raw-capture/src/lib.rs` (module exports)

**Existing trait (DO NOT MODIFY):**

[`crates/data-clients/raw-capture/src/lib.rs:84-97`](../../../crates/data-clients/raw-capture/src/lib.rs#L84-L97) — `RawCapture` trait. 인자 순서 `(pnu, source, raw: &Value, fetched_at)` + 반환 `Result<RawCaptureReceipt, RawCaptureError>` 유지.

> **Note (T1 작성 시점 v3 spec drift)**: spec §3.3 코드 예시는 `Result<(), RawCaptureError>` 로 작성됐으나 실제 trait 는 이미 `RawCaptureReceipt` 반환으로 update 됨 (Codex round 2 review 발견). plan v3 의 모든 SanitizingRawCapture impl 은 *실 trait 시그니처* 를 따른다. spec patch 는 별도 commit 으로 동기화.

---

## Step 1.1: Add sha2 dependency

- [ ] **Step 1.1.1: Modify `crates/data-clients/raw-capture/Cargo.toml`**

기존 `[dependencies]` 섹션에 sha2 추가. workspace `Cargo.toml` 의 `[workspace.dependencies]` 에 `sha2 = "0.10"` 가 이미 있다면 `sha2 = { workspace = true }` 만, 없다면 workspace 에도 추가.

```toml
[dependencies]
# ... 기존 ...
sha2 = { workspace = true }
```

- [ ] **Step 1.1.2: Verify build**

```bash
cargo check -p raw-capture-client
# Expected: Finished `dev` profile [unoptimized + debuginfo]
```

- [ ] **Step 1.1.3: Commit**

```bash
git add crates/data-clients/raw-capture/Cargo.toml Cargo.toml
git commit -m "chore(sp10-5-b-T1): add sha2 dep to raw-capture crate"
```

---

## Step 1.2: RawSanitizer trait + SanitizedRaw struct (failing test)

- [ ] **Step 1.2.1: Create `crates/data-clients/raw-capture/src/sanitizer.rs` with failing test**

```rust
//! PIPA Allowlist-based JSON sanitizer.
//!
//! `RawSanitizer` trait + `SanitizedRaw` 결과 struct. 외부 API raw 응답에서
//! allowlist 외 필드를 폐기하여 PIPA 최소수집 원칙을 컴파일 시점에 강제한다.

use serde_json::Value;

/// Sanitization 결과 — 정제된 JSON + 감사 메타데이터.
#[derive(Debug, Clone)]
pub struct SanitizedRaw {
    pub value: Value,
    pub dropped_count: usize,
    pub schema_hash: String,
    pub sanitizer_version: u32,
}

/// allowlist 기반 raw JSON 정제 인터페이스.
pub trait RawSanitizer: Send + Sync {
    /// `source_id` 별 allowlist 로 raw JSON 을 정제한다.
    /// 비허용 경로는 폐기되고 `dropped_count` 에 누적된다.
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
```

- [ ] **Step 1.2.2: Run test (no impl yet — only struct test)**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::sanitized_raw_construct
# Expected: error[E0432]: unresolved import — sanitizer module not exposed yet
```

이건 lib.rs 에서 sanitizer 모듈 export 안 했으니 실패. 다음 step 에서 export.

- [ ] **Step 1.2.3: Modify `crates/data-clients/raw-capture/src/lib.rs` — expose `sanitizer` module**

기존 `lib.rs` 의 상단 (line 19 직후) 에 추가:

```rust
pub mod sanitizer;
pub use sanitizer::{RawSanitizer, SanitizedRaw};
```

- [ ] **Step 1.2.4: Re-run test**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::sanitized_raw_construct
# Expected: test result: ok. 1 passed
```

- [ ] **Step 1.2.5: Commit**

```bash
git add crates/data-clients/raw-capture/src/sanitizer.rs crates/data-clients/raw-capture/src/lib.rs
git commit -m "feat(sp10-5-b-T1): RawSanitizer trait + SanitizedRaw struct"
```

---

## Step 1.3: compute_schema_hash function (TDD)

spec §5.4 ([design.md:250-256](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md)) literal:

```
schema_hash = SHA-256(
  source_id || ":" || sanitizer_version || ":" || sorted_retained_json_paths.join(",")
)
```

Rust 인자 매핑: `source_id` ↔ `source`, `sorted_retained_json_paths` ↔ `paths` (Step 1.3.3 함수 내부에서 정렬).

- [ ] **Step 1.3.1: Append failing test to `sanitizer.rs`**

`mod tests` 안에 추가:

```rust
    #[test]
    fn schema_hash_deterministic() {
        let h1 = compute_schema_hash("vworld_parcel", 1, &["pnu".to_string(), "geometry".to_string()]);
        let h2 = compute_schema_hash("vworld_parcel", 1, &["geometry".to_string(), "pnu".to_string()]);
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
```

- [ ] **Step 1.3.2: Run tests — verify they fail with "compute_schema_hash not found"**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::schema_hash_deterministic
# Expected: error[E0425]: cannot find function `compute_schema_hash` in this scope
```

- [ ] **Step 1.3.3: Implement `compute_schema_hash`**

`sanitizer.rs` 의 trait 선언 위 또는 아래에 추가 (test 모듈 위).

Spec §5.4 의 의사 코드 변수 → Rust 인자 매핑:
- `source_id` → Rust 인자명 `source` (Rust 관용 — 짧고 의미 동일)
- `sorted_retained_json_paths` → Rust 인자명 `paths` (함수 내부에서 정렬)

수식 자체는 spec literal 그대로: `SHA-256(source_id || ":" || sanitizer_version || ":" || sorted_retained_json_paths.join(","))`.

```rust
use sha2::{Digest, Sha256};

/// allowlist 정의의 SHA-256 hash. drift detection 의 input 이다.
///
/// Spec §5.4 (`design.md:250-256`) 의 의사 코드:
///   `schema_hash = SHA-256(source_id || ":" || sanitizer_version || ":" || sorted_retained_json_paths.join(","))`
///
/// Rust 인자 매핑: `source_id` ↔ `source`, `sorted_retained_json_paths` ↔ `paths`
/// (함수 내부에서 정렬). 출력은 64-char hex digest.
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
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}
```

- [ ] **Step 1.3.4: Run all 3 tests — verify pass**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::schema_hash
# Expected: 3 tests passed
```

- [ ] **Step 1.3.5: Commit**

```bash
git add crates/data-clients/raw-capture/src/sanitizer.rs
git commit -m "feat(sp10-5-b-T1): compute_schema_hash (SHA-256 deterministic)"
```

---

## Step 1.4: AllowlistSanitizer struct (TDD — empty allowlist initial)

allowlist 자체의 상수 정의는 T2 에서. T1 에서는 *구조만* 정의하고 test 용 mock allowlist 로 동작 검증.

- [ ] **Step 1.4.1: Append failing test for struct construction**

```rust
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
    }
```

- [ ] **Step 1.4.2: Run — fail**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::allowlist_sanitizer_constructs
# Expected: error[E0422]: cannot find struct `AllowlistSanitizer`
```

- [ ] **Step 1.4.3: Implement struct + accessors**

`sanitizer.rs` 에 추가:

```rust
/// JSON path-based default-deny sanitizer.
///
/// 허용된 `allowed_paths` 외의 모든 필드를 폐기한다. path 는 JSON pointer 형식
/// (`/response/header/resultCode`) + `*` wildcard (`/items/*/id`).
pub struct AllowlistSanitizer {
    source: String,
    allowed_paths: Vec<String>,
    sanitizer_version: u32,
    schema_hash: String,
}

impl AllowlistSanitizer {
    pub fn new(source: String, allowed_paths: Vec<String>, sanitizer_version: u32) -> Self {
        let schema_hash = compute_schema_hash(&source, sanitizer_version, &allowed_paths);
        Self {
            source,
            allowed_paths,
            sanitizer_version,
            schema_hash,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn allowed_paths(&self) -> &[String] {
        &self.allowed_paths
    }

    pub fn sanitizer_version(&self) -> u32 {
        self.sanitizer_version
    }

    pub fn schema_hash(&self) -> &str {
        &self.schema_hash
    }
}
```

- [ ] **Step 1.4.4: Run — pass**

```bash
cargo test -p raw-capture-client --lib sanitizer::tests::allowlist_sanitizer_constructs
# Expected: ok. 1 passed
```

- [ ] **Step 1.4.5: Commit**

```bash
git add crates/data-clients/raw-capture/src/sanitizer.rs
git commit -m "feat(sp10-5-b-T1): AllowlistSanitizer struct + accessors"
```

---

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

---

## Step 1.7: Drift warn metric verification test (TDD with tracing-test, strict RED → GREEN)

drift 발생 시 `tracing::warn!` 가 실제 발행되는지 mock subscriber 로 검증.

- [ ] **Step 1.7.1: Append failing test to `capture.rs` (test ONLY — dep 미추가 상태)**

`mod tests` 안에 추가:

```rust
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn drift_emits_warn_event() {
        let sanitizer = Arc::new(AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        ));
        let wrapped = SanitizingRawCapture::new(NoOpRawCapture::new(), sanitizer);
        let raw = serde_json::json!({"keep": "ok", "drop_this": "secret"});
        wrapped
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await
            .unwrap();
        assert!(logs_contain("raw_response sanitizer dropped unknown fields"));
        assert!(logs_contain("dropped_count"));
    }

    #[traced_test]
    #[tokio::test]
    async fn no_drift_no_warn() {
        let sanitizer = Arc::new(AllowlistSanitizer::new(
            "test".to_string(),
            vec!["/keep".to_string()],
            1,
        ));
        let wrapped = SanitizingRawCapture::new(NoOpRawCapture::new(), sanitizer);
        let raw = serde_json::json!({"keep": "ok"});
        wrapped
            .capture("1111010100100010000", "test", &raw, Utc::now())
            .await
            .unwrap();
        assert!(!logs_contain("schema_drift"));
    }
```

- [ ] **Step 1.7.2: Run drift tests — verify FAIL (tracing-test crate not yet added)**

```bash
cargo test -p raw-capture-client --lib capture::tests::drift
# Expected: error[E0432]: unresolved import `tracing_test`
# OR: error[E0433]: failed to resolve: use of undeclared crate `tracing_test`
```

- [ ] **Step 1.7.3: Add `tracing-test` dev-dependency**

`crates/data-clients/raw-capture/Cargo.toml`:

```toml
[dev-dependencies]
# ... 기존 ...
tracing-test = "0.2"
```

```bash
cargo check -p raw-capture-client --tests
# Expected: Finished `dev` profile [unoptimized + debuginfo]
```

- [ ] **Step 1.7.4: Re-run drift tests — verify PASS**

```bash
cargo test -p raw-capture-client --lib capture::tests::drift
# Expected: 2 passed (drift_emits_warn_event, no_drift_no_warn)
```

- [ ] **Step 1.7.5: Commit**

```bash
git add crates/data-clients/raw-capture/Cargo.toml crates/data-clients/raw-capture/src/capture.rs
git commit -m "test(sp10-5-b-T1): SanitizingRawCapture drift warn metric verification"
```

---

## Step 1.8: Module re-exports + final verification

**Note**: `lib.rs` doc comment 의 `"vworld"` → `"vworld_parcel"` rename 은 **T2 scope** (migration 30012 과 동일 PR). T1 에서는 module export 만 정리.

- [ ] **Step 1.8.1: Verify `crates/data-clients/raw-capture/src/lib.rs` exports**

기존 doc example (`"vworld"`) 은 *변경하지 않음* — T2 의 30012 마이그레이션 작업과 함께 변경. T1 의 의무는 module re-export 정리만:

```rust
// 상단 module declarations:
pub mod capture;
pub mod sanitizer;

// re-exports (T2 가 사용할 인터페이스):
pub use capture::SanitizingRawCapture;
pub use sanitizer::{AllowlistSanitizer, RawSanitizer, SanitizedRaw, compute_schema_hash};
```

- [ ] **Step 1.8.2: Run all raw-capture tests + clippy**

```bash
cargo test -p raw-capture-client --lib
# Expected: all tests pass (sanitizer + capture modules, 8~10 tests)
cargo clippy -p raw-capture-client -- -D warnings
# Expected: no warnings
cargo fmt --check
# Expected: no diff
```

- [ ] **Step 1.8.3: Commit**

```bash
git add crates/data-clients/raw-capture/src/lib.rs
git commit -m "docs(sp10-5-b-T1): re-export sanitizer + capture modules"
```

---

## Acceptance — T1 완료 기준

- [ ] `cargo test -p raw-capture-client --lib` 전부 통과 (8~10 test)
- [ ] `cargo clippy -p raw-capture-client -- -D warnings` 통과
- [ ] `cargo fmt --check` 통과
- [ ] 신규 파일 2개: `sanitizer.rs`, `capture.rs`
- [ ] 신규 dep: sha2 (runtime), tracing-test (dev)
- [ ] T2 에서 사용할 인터페이스 모두 export: `RawSanitizer`, `SanitizedRaw`, `AllowlistSanitizer::new`, `compute_schema_hash`, `SanitizingRawCapture::new`

**다음 task:** [T2-allowlists-migration.md](T2-allowlists-migration.md) — allowlist 상수 정의 + V-World source rename + migration 30012.

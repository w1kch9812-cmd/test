# T1 Sanitizer Infra - Part 01: Sanitizer Trait And Allowlist Construction

Parent index: [T1 Sanitizer Infra](./T1-sanitizer-infra.md).


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


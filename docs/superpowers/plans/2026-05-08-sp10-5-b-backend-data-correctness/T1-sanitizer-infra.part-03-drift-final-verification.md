# T1 Sanitizer Infra - Part 03: Drift Metric And Final Verification

Parent index: [T1 Sanitizer Infra](./T1-sanitizer-infra.md).

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

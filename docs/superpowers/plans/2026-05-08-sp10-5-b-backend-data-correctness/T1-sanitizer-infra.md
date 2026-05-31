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

## Plan Parts

Detailed step bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Sanitizer Trait And Allowlist Construction](./T1-sanitizer-infra.part-01-trait-allowlist.md)
- [Part 02 - Sanitization And Capture Wrapper](./T1-sanitizer-infra.part-02-sanitize-capture.md)
- [Part 03 - Drift Metric And Final Verification](./T1-sanitizer-infra.part-03-drift-final-verification.md)

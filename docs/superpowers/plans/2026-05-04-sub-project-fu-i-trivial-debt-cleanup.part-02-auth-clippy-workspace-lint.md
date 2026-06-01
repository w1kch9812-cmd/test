# Sub-project FU-i Trivial Debt Cleanup - Part 02: Auth Clippy And Workspace Lint

Parent index: [Sub-project FU-i Trivial Debt Cleanup](./2026-05-04-sub-project-fu-i-trivial-debt-cleanup.md).

## Phase B: 코드 lint 정리

### Task 2: FU 18 — Auth verifier clippy 빚

**Files:**
- Modify: `crates/auth/src/verifier.rs`

- [ ] **Step 1: 현재 lint 위반 식별**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo clippy -p auth --all-features --all-targets -- -D warnings 2>&1 | grep -E "panic|manual_let_else" | head -20
```

발생 위치 파일/라인 메모. 일반적으로:
- `clippy::panic` — production code 에 `panic!()` 직접 호출 (test 외부)
- `clippy::manual_let_else` — `let x = match y { Some(v) => v, None => return ... }` 패턴

- [ ] **Step 2: `clippy::panic` 수정**

위치별로 `panic!()` 을 다음 중 하나로 변경:
- `Result::Err(...)` 반환
- `unreachable!()` → `Err(AuthError::Database("invariant violated".into()))` 또는 비슷한 도메인 에러

예시 (`verifier.rs` 안 가상 위치):
```rust
// Before:
let kid = header.kid.unwrap_or_else(|| panic!("kid missing"));
// After:
let kid = header.kid.ok_or(AuthError::UnknownKey)?;
```

- [ ] **Step 3: `clippy::manual_let_else` 수정**

예시:
```rust
// Before:
let token_str = match header.to_str() {
    Ok(s) => s,
    Err(_) => return Err(AuthError::InvalidFormat),
};
// After:
let Ok(token_str) = header.to_str() else {
    return Err(AuthError::InvalidFormat);
};
```

- [ ] **Step 4: 로컬 검증**

```bash
cargo clippy -p auth --all-features --all-targets -- -D warnings
cargo test -p auth --lib
```

기존 39 unit tests (CI fix 후) 모두 그린 유지. clippy 경고 0.

- [ ] **Step 5: Commit + push**

```bash
git add crates/auth/src/verifier.rs
git commit -m "fix(sp-fu-i-t2): close FU 18 — auth verifier clippy::panic + manual_let_else

verifier.rs 의 pre-existing clippy 빚 정리 (SP3 잔재):
- panic!() → AuthError::* Result 반환
- match { Some(v) => v, None => return } → let-else

cargo clippy --all-targets -D warnings 통과. 기존 39 unit tests 그린 유지."
git push
```

CI 그린 확인.

---

## Phase C: workspace lint 강화

### Task 3: FU 26 — `clippy::disallowed_types` reqwest 차단

**Files:**
- Modify: `clippy.toml` (root)

- [ ] **Step 1: 현재 `clippy.toml` 확인**

```bash
cd c:/Users/User/Desktop/gongzzang_2
cat clippy.toml
```

기존:
```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 5
type-complexity-threshold = 250
too-many-lines-threshold = 100
```

- [ ] **Step 2: `disallowed-types` 추가**

`clippy.toml` 에 추가:
```toml
disallowed-types = [
    { path = "reqwest::Client", reason = "use circuit-breaker::Breaker wrapping reqwest, not direct (FU 26)" },
    { path = "reqwest::blocking::Client", reason = "blocking client 금지, async + Breaker 사용 (FU 26)" },
]
```

- [ ] **Step 3: 예외 crate 식별 및 처리**

`reqwest::Client` 를 *legitimately* 사용하는 crate (HTTP client 그 자체):
- `crates/data-clients/vworld/`
- `crates/data-clients/data-go-kr/`
- `crates/data-clients/raw-capture/` (있다면)
- `crates/auth/` (JWKS fetcher)
- `services/api/` (main.rs 의 reqwest 인스턴스)
- `services/outbox-publisher/` (있다면)

각 crate 에 crate-level allow 추가:

```bash
grep -rn "reqwest::Client\|reqwest::blocking" crates/ services/ --include="*.rs" 2>/dev/null | grep -v test | head -20
```

각 식별된 파일의 모듈 doc-comment 다음 또는 적절한 위치에:
```rust
// 또는 crate root (lib.rs / main.rs):
#![allow(clippy::disallowed_types)] // FU 26 — 본 crate 는 reqwest 직접 사용 허용 (HTTP client wrapper)
```

또는 더 좁게 (해당 import 위에):
```rust
#[allow(clippy::disallowed_types)]
use reqwest::Client;
```

- [ ] **Step 4: 로컬 검증**

```bash
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

- 예외 crate: allow 통과
- 비예외 crate: `reqwest::Client` 사용 시 lint 발생 (현재는 비예외에서 사용 안 할 것)

만약 비예외 crate 가 reqwest 사용한다면:
- 그 사용을 `circuit-breaker::Breaker` 로 wrap 하도록 변경 (별도 task — *본 sub-project 범위 외*)
- 또는 *lint 가 자기 역할 함* 을 인지하고 commit. 발견된 새 위반은 별도 sub-project 로

- [ ] **Step 5: Commit + push**

```bash
git add clippy.toml crates/<예외-crate>/src/{lib,main}.rs ...
git commit -m "feat(sp-fu-i-t3): close FU 26 — clippy::disallowed_types reqwest::Client 차단

workspace clippy.toml 에 disallowed-types 추가:
- reqwest::Client (async)
- reqwest::blocking::Client

예외 crate 에 crate-level allow 추가 (HTTP client wrapper 역할):
- data-clients/vworld, data-clients/data-go-kr, [기타 식별된 crate]
- crates/auth (JWKS fetcher)
- services/api, services/outbox-publisher

비예외 crate 가 reqwest::Client 직접 사용 시 lint 가 차단 → 모든 외부 HTTP
호출이 circuit-breaker::Breaker 통과 강제 (Sentry alert + retry 일관)."
git push
```

CI 그린 확인.

---

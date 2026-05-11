# T5: Building Reader Live Wiring

**Goal:** `services/api/src/main.rs:390-413` 의 `Arc::new(NoOpBuildingRegisterReader)` 를 `Arc::new(DataGoKrBuildingReader::new(client, dual_tier_capture))` 로 swap. 기존 `has_key` 분기 + `is_production` fail-fast panic 패턴은 그대로 유지.

**Spec SSOT:** §1.5 (Building reader live), §11 통합 변경 표, §13 T5 ([design doc](../../specs/2026-05-08-sp10-5-b-backend-data-correctness-design.md))

**T3 inputs:** `gongzzang_db::PgPiiVaultCapture`, `raw_capture_client::{DualTierCapture, SanitizingRawCapture, AllowlistSanitizer}`.

**T4 inputs:** `services/api::cleanup::CleanupTask` (이미 등록됨).

**Files:**

- Modify: `services/api/src/main.rs` (lines 210-221 V-World wiring, 390-413 building wiring)
- Modify: `services/api/src/state.rs` (T7 에서 신규 — 일단 placeholder field 추가)

**Existing code refs:**

- [`services/api/src/main.rs:210-221`](../../../services/api/src/main.rs#L210-L221) — V-World capture wire (audit-fix `b784e76` 이후 `R2RawCapture` 직접 주입 가능성 — 실제 코드 확인 필수)
- [`services/api/src/main.rs:390-413`](../../../services/api/src/main.rs#L390-L413) — `NoOpBuildingRegisterReader` 주입 위치
- [`crates/data-clients/data-go-kr/src/building_register/reader.rs`](../../../crates/data-clients/data-go-kr/src/building_register/reader.rs) — `DataGoKrBuildingReader::new(client, raw_capture)` 시그니처

---

## Step 5.1: 사전 검증 — 실제 main.rs 코드 상태 read

T5 작업 전 spec 의 line 추정 vs 실제 코드를 read 해서 *정확한 wiring 위치* 확인. spec line 번호가 stale 일 수 있음 (T1~T4 commit 후 line shift).

- [ ] **Step 5.1.1: Locate building reader injection**

```bash
grep -n "NoOpBuildingRegisterReader\|DataGoKrBuildingReader" services/api/src/main.rs
# Expected: line N 에 `Arc::new(NoOpBuildingRegisterReader)` 또는 has_key 분기
```

- [ ] **Step 5.1.2: Locate V-World capture wiring**

```bash
grep -n "VWorldParcelReader\|raw_capture" services/api/src/main.rs | head -20
# Expected: V-World reader 가 raw_capture argument 받는 line
```

- [ ] **Step 5.1.3: Locate has_key + fail_fast_production pattern**

```bash
grep -n "has_key\|fail_fast_production\|DATA_GO_KR_API_KEY" services/api/src/main.rs
# Expected: `let has_key = env::var("DATA_GO_KR_API_KEY").is_ok();`
#           `if !has_key && is_production { fail_fast_production(...) }`
```

위 3 검색 결과를 다음 step 의 *기준 line 번호* 로 사용 (spec 의 390-413 등은 추정값).

---

## Step 5.2: Construct DualTierCapture wiring helper (TDD-friendly factory)

main.rs 안에 wiring helper 함수로 분리 — T6 의 admin endpoint + T7 의 integration test 가 동일 wiring 사용 가능.

- [ ] **Step 5.2.1: Add wiring helper to `services/api/src/main.rs`**

main.rs 상단 (또는 별도 module `wiring.rs`) 에 추가:

```rust
use aws_sdk_kms::Client as KmsClient;
use chrono::Duration as ChronoDuration;
use gongzzang_db::PgPiiVaultCapture;
use raw_capture_client::{AllowlistSanitizer, DualTierCapture, RawCapture, SanitizingRawCapture};
use sqlx::PgPool;
use std::sync::Arc;

/// source 별 DualTierCapture (Tier 1 sanitized + Tier 2 vault) 합성체 생성.
///
/// Tier 1 (sanitized) — 기존 production raw sink (R2RawCapture 또는
/// 환경에 따라 PgRawCapture/NoOpRawCapture). caller 가 inner 를 제공.
/// Tier 2 (vault) — KMS envelope encrypted PgPiiVaultCapture.
pub fn build_dual_tier_capture<S>(
    sanitized_sink: S,
    pool: PgPool,
    kms: Arc<KmsClient>,
    kms_key_id: String,
    source: &str,
) -> Result<Arc<dyn RawCapture>, raw_capture_client::SanitizerError>
where
    S: RawCapture + Send + Sync + 'static,
{
    let sanitizer = Arc::new(AllowlistSanitizer::for_source(source)?);
    let sanitizing_tier1 = SanitizingRawCapture::new(sanitized_sink, sanitizer);
    let vault_tier2 = PgPiiVaultCapture::new(
        pool,
        kms,
        kms_key_id,
        ChronoDuration::days(30),
    );
    Ok(Arc::new(DualTierCapture::new(sanitizing_tier1, vault_tier2)))
}
```

- [ ] **Step 5.2.2: Verify compiles**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 5.2.3: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-5-b-T5): build_dual_tier_capture wiring helper"
```

---

## Step 5.3: V-World capture wire swap (main.rs lines 210-221)

기존 production sink (R2RawCapture / PgRawCapture / NoOpRawCapture 중 어느 것) 를 `build_dual_tier_capture` 의 sanitized_sink 인자로 전달.

- [ ] **Step 5.3.1: Modify main.rs V-World wiring section**

기존 (예시 — Step 5.1.2 의 실제 line 으로 정정):

```rust
let raw_capture: Arc<dyn RawCapture> = Arc::new(R2RawCapture::new(r2_client.clone()));
let vworld_reader = VWorldParcelReader::new(vworld_client, raw_capture);
```

→

```rust
let sanitized_sink = R2RawCapture::new(r2_client.clone());  // 기존 production sink
let vworld_capture = build_dual_tier_capture(
    sanitized_sink,
    pool.clone(),
    kms_client.clone(),
    kms_key_id.clone(),
    raw_capture_client::sources::vworld_parcel::SOURCE_ID,
)
.expect("vworld_parcel allowlist 는 항상 등록되어 있어야 함");
let vworld_reader = VWorldParcelReader::new(vworld_client, vworld_capture);
```

- [ ] **Step 5.3.2: Verify compile**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 5.3.3: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-5-b-T5): V-World wiring → DualTierCapture (sanitized + vault)"
```

---

## Step 5.4: Building reader live swap (main.rs lines 390-413)

`NoOpBuildingRegisterReader` → 실 `DataGoKrBuildingReader`. `has_key` 분기 + `is_production` fail-fast 패턴 *유지*.

- [ ] **Step 5.4.1: Modify main.rs building reader injection**

기존 (예시 — Step 5.1.1/5.1.3 의 실제 line 으로 정정):

```rust
let building_reader: Arc<dyn BuildingRegisterReader> = if has_key {
    // 키가 있는데도 NoOp 사용 — BLOCKER (audit 발견 v3)
    Arc::new(NoOpBuildingRegisterReader)
} else {
    if is_production {
        fail_fast_production("DATA_GO_KR_API_KEY missing in production");
    }
    Arc::new(NoOpBuildingRegisterReader)
};
```

→

```rust
let building_reader: Arc<dyn BuildingRegisterReader> = if has_key {
    let api_key = env::var("DATA_GO_KR_API_KEY").expect("has_key guard 가 보장");
    let client = DataGoKrClient::new(api_key);
    let sanitized_sink = R2RawCapture::new(r2_client.clone());
    let building_capture = build_dual_tier_capture(
        sanitized_sink,
        pool.clone(),
        kms_client.clone(),
        kms_key_id.clone(),
        raw_capture_client::sources::data_go_kr_building::SOURCE_ID,
    )
    .expect("data_go_kr_building allowlist 는 항상 등록되어 있어야 함");
    Arc::new(DataGoKrBuildingReader::new(client, building_capture))
} else {
    if is_production {
        fail_fast_production("DATA_GO_KR_API_KEY missing in production");
    }
    // dev/staging — NoOp 유지. AppState 에 building_reader_status = "degraded" 표시 (T7)
    Arc::new(NoOpBuildingRegisterReader)
};
```

- [ ] **Step 5.4.2: Verify compile**

```bash
cargo check -p api
# Expected: Finished
```

- [ ] **Step 5.4.3: Run api unit tests (any existing)**

```bash
cargo test -p api --lib
# Expected: existing tests still pass
```

- [ ] **Step 5.4.4: Commit**

```bash
git add services/api/src/main.rs
git commit -m "feat(sp10-5-b-T5): building reader live swap (NoOp → DataGoKrBuildingReader + DualTier)"
```

---

## Step 5.5: AppState placeholder for building_reader_status / vault_kms_status

T7 에서 정식 `AppState` struct + `app_router` factory 분리. T5 에서는 *향후 T7 가 wire 할 status 필드* 만 placeholder. main.rs 안에 임시 변수.

- [ ] **Step 5.5.1: Add status tracking variables to main.rs**

main.rs 의 building_reader 초기화 직후 추가:

```rust
let building_reader_status: &'static str = if has_key { "live" } else { "degraded" };
let vault_kms_status: &'static str = "ok"; // T7 §7.2 AppState::from_env 가 KMS healthcheck 로 정정

// T5 는 임시 로컬 변수 — T7 §7.4 의 AppState struct + app_router(state) 합치 단계에서
// 정식 state field 로 묶임. 본 step 은 부팅 로그 검증만 책임.
tracing::info!(
    target: "api.startup",
    building_reader_status,
    vault_kms_status,
    "service starting with reader/kms status",
);
```

- [ ] **Step 5.5.2: Verify compile + log**

```bash
cargo check -p api
# Expected: Finished
cargo run -p api 2>&1 | grep "api.startup"
# Expected: building_reader_status="live" 또는 "degraded" 로 부팅 로그 출력
```

- [ ] **Step 5.5.3: Commit**

```bash
git add services/api/src/main.rs
git commit -m "chore(sp10-5-b-T5): building/vault status tracking placeholders (T7 finalize)"
```

---

## Acceptance — T5 완료 기준

- [ ] `services/api/src/main.rs` 의 V-World wiring 이 `DualTierCapture` 합성체 사용
- [ ] Building reader 가 `DATA_GO_KR_API_KEY` 있을 시 실 `DataGoKrBuildingReader` 주입
- [ ] `has_key` + `is_production` fail-fast 패턴 변경 없음 (production 에서 키 미설정 시 panic)
- [ ] `build_dual_tier_capture` 헬퍼 export — T6 / T7 가 동일 wiring 패턴 재사용 가능
- [ ] `cargo check -p api` 통과
- [ ] `cargo clippy -p api -- -D warnings` 통과

**TDD 노트**: T5 는 wiring 중심 task 라 단위 RED→GREEN 보다 *통합 검증* 패턴 사용. wiring 의 정합성은 T7 의 통합 테스트 (`sp10_backend_data_correctness.rs::pii_fixture_dropped_in_tier1` + `health_degraded_when_building_reader_noop`) 가 *실 router + 실 wiring* 으로 검증 → T5 acceptance 의 wiring 정상화는 T7 PASS 시점에 *완전 확정*.

**다음 task:** [T6-admin-rbac-audit.md](T6-admin-rbac-audit.md) — Vault admin endpoint + audit log table + ZITADEL RBAC.

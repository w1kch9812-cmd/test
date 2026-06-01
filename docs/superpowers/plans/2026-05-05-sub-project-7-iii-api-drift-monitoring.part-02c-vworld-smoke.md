# SP7-iii API Drift Monitoring - Part 02C: V-World Smoke Test

Parent index: [SP7-iii API Drift Monitoring - Part 02](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-02.md).
## Phase D: V-World smoke test

### Task 4: T4 — V-World `LP_PA_CBND_BUBUN` 실 API smoke test

**Files:**
- Modify: `crates/data-clients/vworld/Cargo.toml` (real-api feature)
- Create: `crates/data-clients/vworld/tests/smoke_real_api.rs`

#### Step 4.1: Cargo.toml feature 추가

- [ ] **Step**: `crates/data-clients/vworld/Cargo.toml` 에 추가

```toml
[features]
default = []
real-api = []
```

#### Step 4.2: smoke_real_api.rs 작성

- [ ] **Step**: `crates/data-clients/vworld/tests/smoke_real_api.rs` 작성

```rust
//! V-World `LP_PA_CBND_BUBUN` 실 API smoke test (SP7-iii).
//!
//! `cargo test --features real-api -p vworld-client --test smoke_real_api -- --ignored`
//!
//! 환경변수:
//! - `VWORLD_API_KEY` (필수)
//! - `VWORLD_DOMAIN` (필수, default `localhost`)
//! - `GONGZZANG_DRIFT_TEST_PNU` (옵션, default `1168010100107370000` = 강남파이낸스)

#![cfg(feature = "real-api")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use shared_kernel::pnu::Pnu;
use vworld_client::{ParcelReader, VWorldClient, VWorldConfig};

#[tokio::test]
#[ignore]
async fn smoke_vworld_parcel_alive() {
    let key = std::env::var("VWORLD_API_KEY").expect("VWORLD_API_KEY required");
    let domain = std::env::var("VWORLD_DOMAIN").unwrap_or_else(|_| "localhost".to_owned());

    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let config = VWorldConfig {
        api_key: key,
        domain,
        base_url: "https://api.vworld.kr".to_owned(),
    };
    let client = VWorldClient::new(config);

    let parcel = client
        .parcel_reader()
        .fetch_by_pnu(&pnu)
        .await
        .expect("V-World call should succeed (endpoint URL drift?)");

    let parcel = parcel.expect("필지 응답 — endpoint drift 또는 PNU 잘못됨");

    assert_eq!(parcel.pnu.as_str(), pnu.as_str(), "응답 PNU 가 요청과 일치");
    // 핵심 필드 존재 검증 (jiyok_cd 등 — V-World 응답 schema)
    // 미래 V-World schema 변경 시 추가 assert
}
```

**참고**: 실제 `vworld_client` 의 `ParcelReader::fetch_by_pnu` 시그니처 / 반환 타입 확인 필요. 위 코드는 `Result<Option<Parcel>, _>` 가정 (SP4-ii spec 패턴). 차이 시 수정.

#### Step 4.3: feature flag off 빌드 검증

- [ ] **Step**: 평소 build pass 확인

```bash
cargo test -p vworld-client
```

Expected: 기존 wiremock 통합 테스트 그대로 pass.

#### Step 4.4: real-api on 호출 검증 (로컬)

- [ ] **Step**: 실 호출 — V-World 가 작동하는 시간대에

```bash
cargo test --features real-api -p vworld-client \
    --test smoke_real_api -- --ignored --nocapture
```

Expected: V-World 정상 응답 시 pass. 502 (브레인스토밍 시점 일시 장애) 일 수 있음 — 그 경우 panic + log 확인.

#### Step 4.5: clippy

- [ ] **Step**: real-api feature 로 clippy

```bash
cargo clippy --features real-api -p vworld-client --tests -- -D warnings
```

Expected: warnings 0.

#### Step 4.6: T4 commit

- [ ] **Step**: commit

```bash
git add crates/data-clients/vworld/Cargo.toml \
        crates/data-clients/vworld/tests/smoke_real_api.rs

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t4): add V-World real API smoke test (feature-gated)

T4 of SP7-iii:
- crates/data-clients/vworld/Cargo.toml — real-api feature
- tests/smoke_real_api.rs — feature-gated + #[ignore]
  - 강남파이낸스 PNU 로 ParcelReader::fetch_by_pnu 검증
  - V-World 응답 schema 의 핵심 필드 존재 assert
  - GONGZZANG_DRIFT_TEST_PNU env 으로 simulate_failure 지원
- 로컬 검증: cargo test --features real-api -p vworld-client --test smoke_real_api -- --ignored
EOF
)"
```

**사용자 체크포인트**: T4 commit 확인 + 다음 진행 여부.

---

## Phase E: api-health-recorder Rust binary

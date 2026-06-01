# SP7-iii API Drift Monitoring - Part 02B: data.go.kr Smoke Test

Parent index: [SP7-iii API Drift Monitoring - Part 02](./2026-05-05-sub-project-7-iii-api-drift-monitoring.part-02.md).
## Phase C: data.go.kr smoke test

### Task 3: T3 — data.go.kr `BldRgstHubService` 실 API smoke test

**Files:**
- Modify: `crates/data-clients/data-go-kr/Cargo.toml` (real-api feature)
- Create: `crates/data-clients/data-go-kr/tests/smoke_real_api.rs`

#### Step 3.1: Cargo.toml 에 real-api feature 추가

- [ ] **Step**: `crates/data-clients/data-go-kr/Cargo.toml` 에 추가

```toml
[features]
default = []
real-api = []
```

#### Step 3.2: smoke_real_api.rs 작성

- [ ] **Step**: `crates/data-clients/data-go-kr/tests/smoke_real_api.rs` 작성

```rust
//! data.go.kr `BldRgstHubService` 실 API smoke test (SP7-iii).
//!
//! `cargo test --features real-api -p data-go-kr-client --test smoke_real_api -- --ignored`
//!
//! 환경변수:
//! - `ODP_SERVICE_KEY` (필수) — data.go.kr 발급 키
//! - `GONGZZANG_DRIFT_TEST_PNU` (옵션, default `1168010100107370000` = 강남파이낸스)
//!   - simulate_failure workflow input 시 `9999999999999999999` 로 override
//!
//! 검증:
//! 1. `BuildingRegisterClient::fetch_title_info` 가 실 API 응답 받음
//! 2. `parse_building_title` 통과 (schema drift 검출)
//! 3. mainPurpsCd 매핑 정상 (강남파이낸스 = `BuildingPurposeCode::Office`)
//! 4. strctCd 매핑 정상 (강남파이낸스 = `BuildingStructureCode::SteelReinforcedConcrete`)

#![cfg(feature = "real-api")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use building_domain::purpose_code::BuildingPurposeCode;
use building_domain::structure_code::BuildingStructureCode;
use chrono::Utc;
use data_go_kr_client::building_register::parser::parse_building_title;
use data_go_kr_client::building_register::BuildingRegisterClient;
use data_go_kr_client::pnu_split::split;
use data_go_kr_client::{DataGoKrClient, DataGoKrConfig};
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

fn dummy_polygon() -> PolygonSrid {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![])).expect("valid")
}

#[tokio::test]
#[ignore]
async fn smoke_data_go_kr_building_register_alive() {
    let key = std::env::var("ODP_SERVICE_KEY").expect("ODP_SERVICE_KEY required");

    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let config = DataGoKrConfig {
        service_key: key,
        base_url: "https://apis.data.go.kr".to_owned(),
    };
    let client = DataGoKrClient::new(config);

    let br = BuildingRegisterClient::new(&client);
    let raw = br
        .fetch_title_info(split(&pnu))
        .await
        .expect("HTTP call should succeed (endpoint URL drift?)");

    let buildings = parse_building_title(&raw, &pnu, &dummy_polygon(), Utc::now())
        .expect("parser should accept response (schema drift?)");

    assert!(
        !buildings.is_empty(),
        "응답 0건 — endpoint drift 또는 PNU 잘못됨 (simulate_failure 의도된 fail?)"
    );

    // 강남파이낸스센터 검증 (default PNU = 1168010100107370000)
    if pnu.as_str() == "1168010100107370000" {
        let b = &buildings[0];
        assert_eq!(
            b.main_purpose_code,
            BuildingPurposeCode::Office,
            "mainPurpsCd 14000 → Office 매핑 검증"
        );
        assert_eq!(
            b.structure_code,
            BuildingStructureCode::SteelReinforcedConcrete,
            "strctCd 42 → SRC 매핑 검증"
        );
    }
}
```

#### Step 3.3: 평소 cargo test 에서 skip 확인

- [ ] **Step**: feature flag 없이 빌드 통과 확인

```bash
cd c:/Users/User/Desktop/gongzzang_2
cargo test -p data-go-kr-client
```

Expected: 47 lib + 6 wiremock + 6 fixture = 59 tests pass.
`smoke_real_api.rs` 의 테스트는 `#![cfg(feature = "real-api")]` 라 빌드 자체 안 됨 (file 내용 0).

#### Step 3.4: real-api feature 로 호출 검증 (로컬)

- [ ] **Step**: 실 API 호출

```bash
# .env 의 ODP_SERVICE_KEY 가 설정돼 있어야 함
cargo test --features real-api -p data-go-kr-client \
    --test smoke_real_api -- --ignored --nocapture
```

Expected: `smoke_data_go_kr_building_register_alive ... ok` (정부 API 정상 시).

#### Step 3.5: simulate_failure 검증 (로컬)

- [ ] **Step**: 잘못된 PNU 로 fail 의도

```bash
GONGZZANG_DRIFT_TEST_PNU=9999999999999999999 cargo test \
    --features real-api -p data-go-kr-client \
    --test smoke_real_api -- --ignored --nocapture
```

Expected: panic — `assert!(!buildings.is_empty())` fail. 메시지: "응답 0건 — endpoint drift 또는 PNU 잘못됨".

#### Step 3.6: cargo clippy 검증 (real-api on)

- [ ] **Step**: clippy

```bash
cargo clippy --features real-api -p data-go-kr-client --tests -- -D warnings
```

Expected: warnings 0.

#### Step 3.7: T3 commit

- [ ] **Step**: commit

```bash
git add crates/data-clients/data-go-kr/Cargo.toml \
        crates/data-clients/data-go-kr/tests/smoke_real_api.rs

git commit -m "$(cat <<'EOF'
feat(sp7-iii-t3): add data.go.kr real API smoke test (feature-gated)

T3 of SP7-iii:
- crates/data-clients/data-go-kr/Cargo.toml — real-api feature 추가
- tests/smoke_real_api.rs — feature-gated + #[ignore]
  - 강남파이낸스 PNU (1168010100107370000) 실 호출
  - parse_building_title 통과 + Office + SRC 매핑 검증
  - GONGZZANG_DRIFT_TEST_PNU env 으로 simulate_failure 지원
  - 평소 cargo test 에서 빌드/실행 X (real-api off)
- 로컬 검증: cargo test --features real-api -p data-go-kr-client --test smoke_real_api -- --ignored
EOF
)"
```

**사용자 체크포인트**: T3 commit 확인 + 다음 진행 여부.

---

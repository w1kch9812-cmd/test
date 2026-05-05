//! V-World `LT_C_UQ111` 실 API smoke test (SP7-iii).
//!
//! 평소 `cargo test` 에서 빌드/실행 X — `#![cfg(feature = "real-api")]` + `#[ignore]`.
//! CI nightly cron (T6 의 `.github/workflows/api-drift-smoke-test.yml`) 또는
//! 로컬 검증:
//! ```bash
//! cargo test --features real-api -p vworld-client \
//!     --test smoke_real_api -- --ignored --nocapture
//! ```
//!
//! 환경변수:
//! - `VWORLD_API_KEY` (필수) — V-World 개발자 센터 발급 키
//! - `VWORLD_DOMAIN` (옵션, default `localhost`) — 등록 도메인 (Referer 검증)
//! - `GONGZZANG_DRIFT_TEST_PNU` (옵션, default `1168010100107370000` = 강남파이낸스)
//!   `simulate_failure` workflow input 시 잘못된 PNU 로 override 됨
//!
//! 검증 (drift 검출):
//! 1. `VWorldClient::fetch_feature_by_pnu` → `VWorldParcelReader::fetch_by_pnu` 가
//!    실 API HTTP 응답 받음 (endpoint URL drift 검출)
//! 2. `parser::parse_parcel` 통과 (schema drift 검출)
//! 3. 응답 PNU 가 요청한 PNU 와 일치 (정렬/필터 drift 검출)

#![cfg(feature = "real-api")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use parcel_domain::reader::ParcelReader;
use shared_kernel::pnu::Pnu;
use vworld_client::{NoOpRawCapture, VWorldClient, VWorldConfig, VWorldParcelReader};

#[tokio::test]
#[ignore = "real API call — requires VWORLD_API_KEY; runs only in CI nightly cron (T6 워크플로우)"]
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
    let client = Arc::new(VWorldClient::new(config));
    let reader = VWorldParcelReader::new(client, Arc::new(NoOpRawCapture::new()));

    let parcel = reader
        .fetch_by_pnu(&pnu)
        .await
        .expect("V-World HTTP+parse should succeed (endpoint/schema drift 의심?)");

    let parcel = parcel
        .expect("필지 응답 None — endpoint drift 또는 PNU 잘못됨 (simulate_failure 의도된 fail?)");

    assert_eq!(
        parcel.pnu.as_str(),
        pnu.as_str(),
        "응답 PNU 가 요청과 일치 (필터 drift 검출)"
    );
}

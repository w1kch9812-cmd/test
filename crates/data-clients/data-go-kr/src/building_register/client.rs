//! 건축물대장 표제부 endpoint 호출 — `getBrTitleInfo`.
//!
//! API URL 형식 (`docs/data-sources/data-go-kr.md`):
//! ```text
//! GET {base_url}/1613000/BldRgstHubService/getBrTitleInfo
//!   ?ServiceKey={key}
//!   &sigunguCd={5}
//!   &bjdongCd={5}
//!   &platGbCd={1}
//!   &bun={4}
//!   &ji={4}
//!   &numOfRows=100
//!   &pageNo=1
//!   &_type=json
//! ```
//!
//! **Endpoint history (FU 41 검증)**: 이전 `BldRgstService_v2` 는 deprecated —
//! HTTP 200 + body `"Unexpected errors"` 반환. 현재 활성 endpoint 는
//! `BldRgstHubService` (실 API 호출로 검증, 2026-05-04 fixture 5건).

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use circuit_breaker::{execute, BreakerError};
use serde_json::Value;
use tracing::instrument;

use crate::client::DataGoKrClient;
use crate::pnu_split::PnuParts;

/// 건축물대장 표제부 endpoint path — base_url 뒤에 붙어 최종 URL 형성.
///
/// `BldRgstHubService` (Hub) 가 현재 활성 endpoint. 이전 `BldRgstService_v2` 는
/// deprecated — HTTP 200 으로 응답하지만 body 가 `"Unexpected errors"` 텍스트라
/// JSON parse 실패. 2026-05-04 실 API 검증 (FU 41) 으로 발견.
const BR_TITLE_PATH: &str = "/1613000/BldRgstHubService/getBrTitleInfo";

/// 건축물대장 표제부 호출 client.
///
/// `DataGoKrClient` 의 reqwest + breaker + policy 를 빌려 사용.
pub struct BuildingRegisterClient<'a> {
    parent: &'a DataGoKrClient,
}

impl<'a> BuildingRegisterClient<'a> {
    /// `DataGoKrClient` 위에서 새 빌딩레지스터 client.
    #[must_use]
    pub const fn new(parent: &'a DataGoKrClient) -> Self {
        Self { parent }
    }

    /// `getBrTitleInfo` 호출 — raw JSON 반환.
    ///
    /// PNU 분해 5 파라미터 + page/row/type. circuit breaker 통과 → timeout /
    /// retry / open 자동 적용.
    ///
    /// # Errors
    ///
    /// - 네트워크 오류 → [`BreakerError::Inner`] (`reqwest::Error`)
    /// - timeout / circuit open → [`BreakerError::Timeout`] / [`BreakerError::Open`]
    #[instrument(skip(self), fields(
        sigungu = %parts.sigungu_cd,
        bjdong = %parts.bjdong_cd,
        bun = %parts.bun,
        ji = %parts.ji,
    ))]
    pub async fn fetch_title_info(
        &self,
        parts: PnuParts<'_>,
    ) -> Result<Value, BreakerError<reqwest::Error>> {
        let base = self.parent.base_url();
        let key = self.parent.service_key();
        // platGbCd 매핑: data.go.kr 은 일반 = "0", 산 = "1" — PNU char 10 ("1"
        // 일반 / "2" 산) 와 다른 표기. 호출 측이 매핑 (FU 41) — 본 함수는 PNU
        // 슬라이스 그대로 넘김. data.go.kr 가 받는 platGbCd 1자리 인 것은 동일.
        let url = format!(
            "{base}{BR_TITLE_PATH}?ServiceKey={key}&sigunguCd={sigungu}&bjdongCd={bjdong}&platGbCd={plat_gb}&bun={bun}&ji={ji}&numOfRows=100&pageNo=1&_type=json",
            sigungu = parts.sigungu_cd,
            bjdong = parts.bjdong_cd,
            plat_gb = parts.plat_gb_cd,
            bun = parts.bun,
            ji = parts.ji,
        );

        execute(
            &self.parent.breaker,
            &self.parent.policy,
            "data_go_kr.getBrTitleInfo",
            || async {
                let resp = self.parent.http.get(&url).send().await?;
                resp.error_for_status()?.json::<Value>().await
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::client::DataGoKrConfig;

    #[test]
    fn building_register_client_borrows_parent() {
        let cfg = DataGoKrConfig {
            service_key: "k".to_owned(),
            base_url: "http://x".to_owned(),
        };
        let parent = DataGoKrClient::new(cfg);
        let _br = BuildingRegisterClient::new(&parent);
    }
}

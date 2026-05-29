//! Platform Core Catalog-backed parcel lookup adapter.
//!
//! `parcel-lookup` owns only the Gongzzang port. This module owns the HTTP
//! integration because Platform Core is an external runtime dependency of the
//! API service, not a domain crate concern.

#![allow(clippy::disallowed_types, clippy::module_name_repetitions)]

use std::sync::Arc;

use async_trait::async_trait;
use circuit_breaker::{execute, Breaker, Policy};
use parcel_lookup::{LookupError, ParcelInfo, ParcelInfoLookup};
use reqwest::StatusCode;
use serde::Deserialize;
use shared_kernel::admin_division::{AdminDivision, EupmyeondongCode, SidoCode, SigunguCode};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::pnu::Pnu;
use thiserror::Error;
use tracing::instrument;

use crate::platform_core_auth::PlatformCoreServiceAuth;

const PLATFORM_CORE_API_BASE_URL_ENV: &str = "PLATFORM_CORE_API_BASE_URL";

/// Platform Core HTTP-backed parcel lookup adapter.
pub struct PlatformCoreParcelInfoLookup {
    base_url: reqwest::Url,
    client: reqwest::Client,
    auth: Option<PlatformCoreServiceAuth>,
    breaker: Breaker,
    policy: Policy,
}

impl PlatformCoreParcelInfoLookup {
    /// Build a Platform Core lookup from an API base URL.
    ///
    /// # Errors
    ///
    /// Returns a config error when the URL is empty, invalid, or the HTTP
    /// client cannot be constructed.
    pub fn new(
        base_url: &str,
        auth: Option<PlatformCoreServiceAuth>,
    ) -> Result<Self, PlatformCoreParcelLookupConfigError> {
        let base_url = parse_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .build()
            .map_err(|source| PlatformCoreParcelLookupConfigError::HttpClient { source })?;

        Ok(Self {
            base_url,
            client,
            auth,
            breaker: Breaker::new(),
            policy: Policy::platform_core_default(),
        })
    }

    fn parcel_url(&self, pnu: &Pnu) -> Result<reqwest::Url, LookupError> {
        self.base_url
            .join(&format!("catalog/v1/parcels/by-pnu/{}", pnu.as_str()))
            .map_err(|error| LookupError::Backend(format!("build Platform Core URL: {error}")))
    }
}

#[async_trait]
impl ParcelInfoLookup for PlatformCoreParcelInfoLookup {
    #[instrument(skip(self), fields(pnu = %pnu.as_str()))]
    async fn lookup_by_pnu(&self, pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError> {
        let url = self.parcel_url(pnu)?;
        let response = execute(
            &self.breaker,
            &self.policy,
            "platform_core.catalog.get_parcel_by_pnu",
            || {
                let client = self.client.clone();
                let url = url.clone();
                let auth = self.auth.clone();
                async move { send_platform_core_get(&client, url, auth.as_ref()).await }
            },
        )
        .await
        .map_err(|error| {
            LookupError::Backend(format!("Platform Core parcel lookup failed: {error}"))
        })?;

        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(LookupError::Backend(format!(
                "Platform Core parcel lookup returned {}",
                response.status()
            )));
        }

        let parcel = response
            .json::<PlatformCoreParcelResponse>()
            .await
            .map_err(|error| LookupError::Parse(error.to_string()))?;

        parcel_info_from_response(pnu, &parcel).map(Some)
    }
}

/// Build a Platform Core lookup behind the trait-object port.
///
/// # Errors
///
/// Returns a config error when `base_url` is invalid.
pub fn build_platform_core_parcel_info_lookup(
    base_url: &str,
    auth: Option<PlatformCoreServiceAuth>,
) -> Result<Arc<dyn ParcelInfoLookup>, PlatformCoreParcelLookupConfigError> {
    Ok(Arc::new(PlatformCoreParcelInfoLookup::new(base_url, auth)?))
}

/// Configuration errors for the Platform Core parcel lookup adapter.
#[derive(Debug, Error)]
pub enum PlatformCoreParcelLookupConfigError {
    /// Required environment value is present but blank.
    #[error("{name} must not be empty")]
    EmptyEnv {
        /// Environment variable name.
        name: &'static str,
    },
    /// Base URL cannot be parsed by the HTTP client.
    #[error("invalid Platform Core API base URL: {0}")]
    InvalidBaseUrl(String),
    /// HTTP client construction failed.
    #[error("build Platform Core HTTP client: {source}")]
    HttpClient {
        /// Underlying HTTP client construction error.
        source: reqwest::Error,
    },
}

#[derive(Debug, Error)]
enum PlatformCoreParcelHttpError {
    #[error("Platform Core parcel lookup HTTP request failed: {source}")]
    Request {
        #[source]
        source: reqwest::Error,
    },
    #[error("Platform Core parcel lookup returned retriable status {status}")]
    RetriableStatus { status: StatusCode },
    #[error("Platform Core parcel lookup service auth failed: {source}")]
    ServiceAuth {
        #[source]
        source: crate::platform_core_auth::PlatformCoreServiceAuthConfigError,
    },
}

#[derive(Debug, Deserialize)]
struct PlatformCoreParcelResponse {
    pnu: String,
    kind: String,
}

async fn send_platform_core_get(
    client: &reqwest::Client,
    url: reqwest::Url,
    auth: Option<&PlatformCoreServiceAuth>,
) -> Result<reqwest::Response, PlatformCoreParcelHttpError> {
    let request = client.get(url);
    let request = if let Some(auth) = auth {
        auth.apply(request)
            .map_err(|source| PlatformCoreParcelHttpError::ServiceAuth { source })?
    } else {
        request
    };
    let response = request
        .send()
        .await
        .map_err(|source| PlatformCoreParcelHttpError::Request { source })?;
    let status = response.status();
    if is_retriable_status(status) {
        return Err(PlatformCoreParcelHttpError::RetriableStatus { status });
    }
    Ok(response)
}

fn is_retriable_status(status: StatusCode) -> bool {
    status.is_server_error() || status == StatusCode::TOO_MANY_REQUESTS
}

fn parse_base_url(base_url: &str) -> Result<reqwest::Url, PlatformCoreParcelLookupConfigError> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(PlatformCoreParcelLookupConfigError::EmptyEnv {
            name: PLATFORM_CORE_API_BASE_URL_ENV,
        });
    }
    let normalized = if trimmed.ends_with('/') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}/")
    };
    reqwest::Url::parse(&normalized)
        .map_err(|error| PlatformCoreParcelLookupConfigError::InvalidBaseUrl(error.to_string()))
}

fn parcel_info_from_response(
    requested_pnu: &Pnu,
    response: &PlatformCoreParcelResponse,
) -> Result<ParcelInfo, LookupError> {
    let response_pnu = Pnu::try_new(&response.pnu)
        .map_err(|error| LookupError::Parse(format!("invalid response PNU: {error}")))?;
    if response_pnu.as_str() != requested_pnu.as_str() {
        return Err(LookupError::Parse(format!(
            "response PNU mismatch: requested={}, response={}",
            requested_pnu.as_str(),
            response_pnu.as_str()
        )));
    }

    Ok(ParcelInfo {
        admin: admin_from_pnu(requested_pnu)?,
        land_use_type: land_use_type_from_platform_core_kind(&response.kind)?,
        zoning: None,
        official_land_price_per_m2: None,
        gosi_year_month: None,
    })
}

fn admin_from_pnu(pnu: &Pnu) -> Result<AdminDivision, LookupError> {
    let sido = SidoCode::try_new(pnu.sido_code())
        .map_err(|error| LookupError::Parse(format!("invalid PNU sido code: {error}")))?;
    let sigungu = SigunguCode::try_new(pnu.sigungu_code())
        .map_err(|error| LookupError::Parse(format!("invalid PNU sigungu code: {error}")))?;
    let eupmyeondong = EupmyeondongCode::try_new(pnu.eupmyeondong_code())
        .map_err(|error| LookupError::Parse(format!("invalid PNU eupmyeondong code: {error}")))?;

    AdminDivision::try_new(sido, sigungu, eupmyeondong)
        .map_err(|error| LookupError::Parse(format!("invalid PNU admin hierarchy: {error}")))
}

fn land_use_type_from_platform_core_kind(kind: &str) -> Result<LandUseType, LookupError> {
    match kind {
        "factory" => Ok(LandUseType::FactorySite),
        "support" => Ok(LandUseType::Building),
        "public" | "river" | "other" => Ok(LandUseType::Other),
        other => Err(LookupError::Parse(format!(
            "unknown Platform Core parcel kind: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    use shared_kernel::land_use_type::LandUseType;

    use crate::platform_core_auth::PlatformCoreServiceAuth;

    use super::*;

    const REQUEST_PNU: &str = "1168010100107370000";
    const OTHER_PNU: &str = "1111010100100010000";

    #[tokio::test]
    async fn lookup_success_maps_platform_core_kind_and_pnu_admin() {
        let base_url = spawn_platform_core_response(
            REQUEST_PNU,
            "HTTP/1.1 200 OK",
            &platform_core_parcel_json(REQUEST_PNU, "factory"),
        );
        let lookup = PlatformCoreParcelInfoLookup::new(&base_url, None).expect("valid base url");
        let pnu = Pnu::try_new(REQUEST_PNU).unwrap();

        let info = lookup.lookup_by_pnu(&pnu).await.unwrap().unwrap();

        assert_eq!(info.admin.sido.as_str(), "11");
        assert_eq!(info.admin.sigungu.as_str(), "11680");
        assert_eq!(info.admin.eupmyeondong.as_str(), "11680101");
        assert_eq!(info.land_use_type, LandUseType::FactorySite);
        assert!(info.zoning.is_none());
        assert!(info.official_land_price_per_m2.is_none());
        assert!(info.gosi_year_month.is_none());
    }

    #[tokio::test]
    async fn lookup_404_returns_none() {
        let base_url = spawn_platform_core_response(
            REQUEST_PNU,
            "HTTP/1.1 404 Not Found",
            r#"{"error":"not found"}"#,
        );
        let lookup = PlatformCoreParcelInfoLookup::new(&base_url, None).expect("valid base url");
        let pnu = Pnu::try_new(REQUEST_PNU).unwrap();

        assert!(lookup.lookup_by_pnu(&pnu).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn lookup_rejects_mismatched_response_pnu() {
        let base_url = spawn_platform_core_response(
            REQUEST_PNU,
            "HTTP/1.1 200 OK",
            &platform_core_parcel_json(OTHER_PNU, "factory"),
        );
        let lookup = PlatformCoreParcelInfoLookup::new(&base_url, None).expect("valid base url");
        let pnu = Pnu::try_new(REQUEST_PNU).unwrap();

        match lookup.lookup_by_pnu(&pnu).await.unwrap_err() {
            LookupError::Parse(message) => assert!(message.contains("response PNU mismatch")),
            other @ LookupError::Backend(_) => panic!("expected parse error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn lookup_sends_platform_core_service_bearer_token() {
        let (base_url, requests) = spawn_platform_core_response_capture(
            REQUEST_PNU,
            "HTTP/1.1 200 OK",
            &platform_core_parcel_json(REQUEST_PNU, "factory"),
        );
        let auth = PlatformCoreServiceAuth::new("platform-core-service-token-32-valid")
            .expect("service auth");
        let lookup =
            PlatformCoreParcelInfoLookup::new(&base_url, Some(auth)).expect("valid base url");
        let pnu = Pnu::try_new(REQUEST_PNU).unwrap();

        lookup.lookup_by_pnu(&pnu).await.unwrap();

        let request = requests
            .recv_timeout(Duration::from_secs(2))
            .expect("captured request");
        assert!(
            request.contains("\r\nauthorization: Bearer platform-core-service-token-32-valid\r\n")
                || request
                    .contains("\r\nAuthorization: Bearer platform-core-service-token-32-valid\r\n"),
            "request missing service bearer token: {request}"
        );
    }

    fn spawn_platform_core_response(expected_pnu: &str, status_line: &str, body: &str) -> String {
        let (base_url, _requests) =
            spawn_platform_core_response_capture(expected_pnu, status_line, body);
        base_url
    }

    fn spawn_platform_core_response_capture(
        expected_pnu: &str,
        status_line: &str,
        body: &str,
    ) -> (String, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let expected_path = format!("GET /catalog/v1/parcels/by-pnu/{expected_pnu} ");
        let status_line = status_line.to_owned();
        let body = body.to_owned();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(
                request.starts_with(&expected_path),
                "request path mismatch: {request}"
            );
            let _ = tx.send(request.to_string());
            let response = format!(
                "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        (format!("http://{addr}"), rx)
    }

    fn platform_core_parcel_json(pnu: &str, kind: &str) -> String {
        format!(
            r#"{{
                "id":"018f2ec8-7f3a-79db-8f7f-3d65f4277f00",
                "complex_id":"018f2ec8-7f3a-79db-8f7f-3d65f4277f01",
                "pnu":"{pnu}",
                "kind":"{kind}",
                "area_m2":1200,
                "version":3,
                "updated_at":"2026-05-28T00:00:00Z"
            }}"#
        )
    }
}

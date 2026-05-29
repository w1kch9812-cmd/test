//! Platform Core Catalog-backed building reader.
//!
//! Gongzzang owns the `/api/buildings` route shape, but Platform Core owns
//! canonical catalog building data. This adapter is the only translation layer
//! between the Platform Core public Catalog API and the Gongzzang route-facing
//! `BuildingRegisterRecord`.

#![allow(clippy::disallowed_types, clippy::module_name_repetitions)]

use std::sync::Arc;

use circuit_breaker::{execute, Breaker, Policy};
use serde::Deserialize;
use shared_kernel::pnu::Pnu;
use thiserror::Error;

use crate::platform_core_auth::PlatformCoreServiceAuth;
use crate::routes::buildings::{
    BuildingRegisterError, BuildingRegisterReader, BuildingRegisterRecord,
};

/// Configuration error for the Platform Core building reader.
#[derive(Debug, Error)]
pub enum PlatformCoreBuildingReaderConfigError {
    /// `PLATFORM_CORE_API_BASE_URL` was set to an empty string.
    #[error("PLATFORM_CORE_API_BASE_URL must not be empty")]
    EmptyBaseUrl,
    /// The HTTP client could not be built.
    #[error("build Platform Core HTTP client: {source}")]
    HttpClient {
        /// Underlying HTTP client construction error.
        source: reqwest::Error,
    },
}

#[derive(Debug, Error)]
enum PlatformCoreBuildingReaderError {
    #[error("Platform Core building lookup HTTP request failed: {source}")]
    Request {
        #[source]
        source: reqwest::Error,
    },
    #[error("Platform Core building lookup returned status {status}")]
    Status { status: reqwest::StatusCode },
    #[error("Platform Core building lookup returned retriable status {status}")]
    RetriableStatus { status: reqwest::StatusCode },
    #[error("Platform Core building lookup service auth failed: {source}")]
    ServiceAuth {
        #[source]
        source: crate::platform_core_auth::PlatformCoreServiceAuthConfigError,
    },
    #[error(
        "Platform Core building stories value is outside Gongzzang route contract: id={id} stories={stories}"
    )]
    InvalidStories { id: String, stories: i16 },
}

/// Building reader that consumes Platform Core Catalog's public HTTP API.
pub struct PlatformCoreBuildingRegisterReader {
    client: reqwest::Client,
    breaker: Breaker,
    policy: Policy,
    auth: Option<PlatformCoreServiceAuth>,
    base_url: String,
}

impl PlatformCoreBuildingRegisterReader {
    /// Creates a Platform Core HTTP-backed building reader.
    ///
    /// # Errors
    ///
    /// Returns an error when `base_url` is empty or the HTTP client cannot be
    /// constructed.
    pub fn new(
        base_url: &str,
        auth: Option<PlatformCoreServiceAuth>,
    ) -> Result<Self, PlatformCoreBuildingReaderConfigError> {
        let base_url = normalize_base_url(base_url)?;
        let client = reqwest::Client::builder()
            .build()
            .map_err(|source| PlatformCoreBuildingReaderConfigError::HttpClient { source })?;
        Ok(Self {
            client,
            breaker: Breaker::new(),
            policy: Policy::platform_core_default(),
            auth,
            base_url,
        })
    }
}

impl BuildingRegisterReader for PlatformCoreBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<Vec<BuildingRegisterRecord>, BuildingRegisterError>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let url = format!(
                "{}/catalog/v1/parcels/by-pnu/{}/buildings",
                self.base_url,
                pnu.as_str()
            );
            let response = execute(
                &self.breaker,
                &self.policy,
                "platform_core.catalog.list_parcel_buildings_by_pnu",
                || {
                    let client = self.client.clone();
                    let url = url.clone();
                    let auth = self.auth.clone();
                    async move { send_platform_core_get(&client, &url, auth.as_ref()).await }
                },
            )
            .await
            .map_err(|source| Box::new(source) as BuildingRegisterError)?;
            let status = response.status();
            if !status.is_success() {
                return Err(Box::new(PlatformCoreBuildingReaderError::Status { status })
                    as BuildingRegisterError);
            }

            let buildings = response
                .json::<Vec<PlatformCoreBuildingResponse>>()
                .await
                .map_err(|source| {
                    Box::new(PlatformCoreBuildingReaderError::Request { source })
                        as BuildingRegisterError
                })?;

            buildings
                .into_iter()
                .map(BuildingRegisterRecord::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|source| Box::new(source) as BuildingRegisterError)
        })
    }
}

/// Builds the Platform Core Catalog-backed building reader.
///
/// # Errors
///
/// Returns an error when `base_url` is invalid for reader construction.
pub fn build_platform_core_building_register_reader(
    base_url: &str,
    auth: Option<PlatformCoreServiceAuth>,
) -> Result<Arc<dyn BuildingRegisterReader>, PlatformCoreBuildingReaderConfigError> {
    Ok(Arc::new(PlatformCoreBuildingRegisterReader::new(
        base_url, auth,
    )?))
}

#[derive(Debug, Deserialize)]
struct PlatformCoreBuildingResponse {
    id: String,
    #[allow(dead_code)]
    parcel_id: String,
    purpose_code: String,
    structure_code: String,
    floor_area_m2: f64,
    stories: i16,
    #[allow(dead_code)]
    built_year: i32,
    #[allow(dead_code)]
    updated_at: String,
}

impl TryFrom<PlatformCoreBuildingResponse> for BuildingRegisterRecord {
    type Error = PlatformCoreBuildingReaderError;

    fn try_from(value: PlatformCoreBuildingResponse) -> Result<Self, Self::Error> {
        let above_ground_floors =
            u8::try_from(value.stories).map_err(|_| Self::Error::InvalidStories {
                id: value.id.clone(),
                stories: value.stories,
            })?;

        Ok(Self {
            id: value.id,
            name: String::new(),
            address: None,
            purpose: value.purpose_code,
            structure: value.structure_code,
            plot_area_m2: None,
            building_area_m2: None,
            building_coverage_ratio: None,
            total_area_m2: value.floor_area_m2,
            floor_area_ratio: None,
            above_ground_floors,
            below_ground_floors: 0,
            height_m: None,
            passenger_elevators: None,
            emergency_elevators: None,
            indoor_self_parking: None,
            outdoor_self_parking: None,
            annex_building_count: None,
            annex_building_area_m2: None,
            permitted_at: None,
            started_at: None,
            approved_at: None,
        })
    }
}

async fn send_platform_core_get(
    client: &reqwest::Client,
    url: &str,
    auth: Option<&PlatformCoreServiceAuth>,
) -> Result<reqwest::Response, PlatformCoreBuildingReaderError> {
    let request = client.get(url);
    let request = if let Some(auth) = auth {
        auth.apply(request)
            .map_err(|source| PlatformCoreBuildingReaderError::ServiceAuth { source })?
    } else {
        request
    };
    let response = request
        .send()
        .await
        .map_err(|source| PlatformCoreBuildingReaderError::Request { source })?;
    let status = response.status();
    if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(PlatformCoreBuildingReaderError::RetriableStatus { status });
    }
    Ok(response)
}

fn normalize_base_url(base_url: &str) -> Result<String, PlatformCoreBuildingReaderConfigError> {
    let normalized = base_url.trim().trim_end_matches('/').to_owned();
    if normalized.is_empty() {
        return Err(PlatformCoreBuildingReaderConfigError::EmptyBaseUrl);
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::Duration;

    use crate::platform_core_auth::PlatformCoreServiceAuth;

    use super::*;

    #[tokio::test]
    async fn reads_buildings_from_platform_core_catalog_by_pnu() {
        let body = r#"
[
  {
    "id": "building-01",
    "parcel_id": "parcel-01",
    "purpose_code": "factory",
    "structure_code": "steel",
    "floor_area_m2": 1234.5,
    "stories": 7,
    "built_year": 2020,
    "updated_at": "2026-05-28T00:00:00Z"
  }
]
"#;
        let (base_url, request_line) = spawn_platform_core_response("HTTP/1.1 200 OK", body);
        let reader = PlatformCoreBuildingRegisterReader::new(&base_url, None).expect("reader");
        let pnu = Pnu::try_new("1168010100107370000").expect("valid pnu");

        let records = reader.list_by_pnu(&pnu).await.expect("records");

        assert_eq!(
            request_line
                .recv_timeout(Duration::from_secs(2))
                .expect("request line"),
            "GET /catalog/v1/parcels/by-pnu/1168010100107370000/buildings HTTP/1.1"
        );
        assert_eq!(
            records,
            vec![BuildingRegisterRecord {
                id: "building-01".to_owned(),
                name: String::new(),
                address: None,
                purpose: "factory".to_owned(),
                structure: "steel".to_owned(),
                plot_area_m2: None,
                building_area_m2: None,
                building_coverage_ratio: None,
                total_area_m2: 1234.5,
                floor_area_ratio: None,
                above_ground_floors: 7,
                below_ground_floors: 0,
                height_m: None,
                passenger_elevators: None,
                emergency_elevators: None,
                indoor_self_parking: None,
                outdoor_self_parking: None,
                annex_building_count: None,
                annex_building_area_m2: None,
                permitted_at: None,
                started_at: None,
                approved_at: None,
            }]
        );
    }

    #[tokio::test]
    async fn returns_error_for_platform_core_non_success_status() {
        let (base_url, _request_line) =
            spawn_platform_core_responses("HTTP/1.1 503 Service Unavailable", "{}", 2);
        let reader = PlatformCoreBuildingRegisterReader::new(&base_url, None).expect("reader");
        let pnu = Pnu::try_new("1168010100107370000").expect("valid pnu");

        let error = reader.list_by_pnu(&pnu).await.expect_err("status error");

        assert!(error.to_string().contains("503"));
    }

    #[tokio::test]
    async fn rejects_platform_core_building_story_count_outside_route_contract() {
        let body = r#"
[
  {
    "id": "building-01",
    "parcel_id": "parcel-01",
    "purpose_code": "factory",
    "structure_code": "steel",
    "floor_area_m2": 1234.5,
    "stories": -1,
    "built_year": 2020,
    "updated_at": "2026-05-28T00:00:00Z"
  }
]
"#;
        let (base_url, _request_line) = spawn_platform_core_response("HTTP/1.1 200 OK", body);
        let reader = PlatformCoreBuildingRegisterReader::new(&base_url, None).expect("reader");
        let pnu = Pnu::try_new("1168010100107370000").expect("valid pnu");

        let error = reader.list_by_pnu(&pnu).await.expect_err("invalid stories");

        assert!(error.to_string().contains("stories"));
    }

    #[tokio::test]
    async fn sends_platform_core_service_bearer_token() {
        let body = r#"
[
  {
    "id": "building-01",
    "parcel_id": "parcel-01",
    "purpose_code": "factory",
    "structure_code": "steel",
    "floor_area_m2": 1234.5,
    "stories": 7,
    "built_year": 2020,
    "updated_at": "2026-05-28T00:00:00Z"
  }
]
"#;
        let (base_url, requests) = spawn_platform_core_request_capture("HTTP/1.1 200 OK", body);
        let auth = PlatformCoreServiceAuth::new("platform-core-service-token-32-valid")
            .expect("service auth");
        let reader =
            PlatformCoreBuildingRegisterReader::new(&base_url, Some(auth)).expect("reader");
        let pnu = Pnu::try_new("1168010100107370000").expect("valid pnu");

        reader.list_by_pnu(&pnu).await.expect("records");

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

    fn spawn_platform_core_response(status_line: &str, body: &str) -> (String, Receiver<String>) {
        spawn_platform_core_responses(status_line, body, 1)
    }

    fn spawn_platform_core_responses(
        status_line: &str,
        body: &str,
        request_count: usize,
    ) -> (String, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let status_line = status_line.to_owned();
        let body = body.to_owned();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for _ in 0..request_count {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut request = [0_u8; 2048];
                let read = stream.read(&mut request).expect("read request");
                let request = String::from_utf8_lossy(&request[..read]);
                tx.send(request.lines().next().unwrap_or_default().to_owned())
                    .expect("send request line");
                let response = format!(
                    "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
        });

        (format!("http://{addr}"), rx)
    }

    fn spawn_platform_core_request_capture(
        status_line: &str,
        body: &str,
    ) -> (String, Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("local addr");
        let status_line = status_line.to_owned();
        let body = body.to_owned();
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 2048];
            let read = stream.read(&mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..read]);
            tx.send(request.to_string()).expect("send request");
            let response = format!(
                "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        (format!("http://{addr}"), rx)
    }
}

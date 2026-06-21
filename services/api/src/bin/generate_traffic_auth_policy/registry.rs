//! Deserialization model for the traffic/auth policy registry SSOT.
//!
//! Mirrors the shape of `docs/architecture/traffic-auth-policy-registry.v1.json`.
//! Only the fields consumed by the generator are modeled; unknown fields are
//! ignored so the hand-edited registry can carry documentation-only keys.

use serde::Deserialize;

/// Schema version the generator is built against.
pub const EXPECTED_SCHEMA_VERSION: &str = "gongzzang.traffic_auth_policy_registry.v1";

/// Root of the combined registry SSOT.
#[derive(Debug, Deserialize)]
pub struct Registry {
    pub schema_version: String,
    #[serde(default)]
    pub public_route_policies: Vec<PublicRoutePolicy>,
    #[serde(default)]
    pub auth_route_policies: Vec<AuthRoutePolicy>,
    #[serde(default)]
    pub page_route_policies: Vec<PageRoutePolicy>,
    #[serde(default)]
    pub route_rate_profiles: Vec<RouteRateProfile>,
    #[serde(default)]
    pub api_proxy_route_policies: Vec<ApiProxyRoutePolicy>,
    #[serde(default)]
    pub backend_route_policies: Vec<BackendRoutePolicy>,
    #[serde(default)]
    pub service_call_policies: Vec<ServiceCallPolicy>,
}

/// Public anonymous map route policy.
#[derive(Debug, Deserialize)]
pub struct PublicRoutePolicy {
    pub id: String,
    pub proxy_path_kind: String,
    pub proxy_path_source: String,
    pub proxy_path: String,
    pub backend_route: String,
    pub methods: Vec<String>,
    pub rate_policy: PublicRatePolicy,
    #[serde(default)]
    pub cache_policy: Option<CachePolicy>,
    #[serde(default)]
    pub single_flight_policy: Option<SingleFlightPolicy>,
    #[serde(default)]
    pub response_budget: Option<ResponseBudget>,
    pub data_exposure_policy: DataExposurePolicy,
    #[serde(default)]
    pub forbidden_request_shapes: Option<Vec<String>>,
}

/// Rate policy block for a public route.
#[derive(Debug, Deserialize)]
pub struct PublicRatePolicy {
    pub key_prefix: String,
    pub limit: i64,
    pub window_seconds: i64,
    pub problem_type: String,
}

/// Cache policy block (only `ttl_seconds` is consumed).
#[derive(Debug, Deserialize)]
pub struct CachePolicy {
    #[serde(default)]
    pub ttl_seconds: Option<i64>,
}

/// Single-flight policy block.
#[derive(Debug, Deserialize)]
pub struct SingleFlightPolicy {
    #[serde(default)]
    pub lock_seconds: Option<i64>,
    #[serde(default)]
    pub wait_attempts: Option<i64>,
    #[serde(default)]
    pub wait_milliseconds: Option<i64>,
}

/// Response budget block.
#[derive(Debug, Deserialize)]
#[allow(clippy::struct_field_names)]
pub struct ResponseBudget {
    #[serde(default)]
    pub max_tile_bytes: Option<i64>,
    #[serde(default)]
    pub max_features: Option<i64>,
    #[serde(default)]
    pub max_mask_ids: Option<i64>,
}

/// Data exposure policy block (only `allowed_data_classes` is consumed).
#[derive(Debug, Deserialize)]
pub struct DataExposurePolicy {
    pub allowed_data_classes: Vec<String>,
}

/// Auth route rate policy.
#[derive(Debug, Deserialize)]
pub struct AuthRoutePolicy {
    pub id: String,
    pub path_source: String,
    pub methods: Vec<String>,
    pub rate_policy: AuthRatePolicy,
}

/// Rate policy block for an auth route.
#[derive(Debug, Deserialize)]
pub struct AuthRatePolicy {
    pub key_prefix: String,
    pub key_strategy: String,
    pub limit: i64,
    pub window_seconds: i64,
    pub problem_type: String,
}

/// Page route gate policy.
#[derive(Debug, Deserialize)]
pub struct PageRoutePolicy {
    pub path_kind: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub path_source: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub prefix_source: Option<String>,
    #[serde(default)]
    pub suffix: Option<String>,
    pub required_roles: Vec<String>,
}

/// Reusable named rate profile.
#[derive(Debug, Deserialize)]
pub struct RouteRateProfile {
    pub id: String,
    pub key_prefix: String,
    pub key_strategy: String,
    pub limit: i64,
    pub window_seconds: i64,
    pub problem_type: String,
}

/// API proxy route policy.
#[derive(Debug, Deserialize)]
pub struct ApiProxyRoutePolicy {
    pub id: String,
    pub target_path_kind: String,
    pub target_path: String,
    pub methods: Vec<String>,
    pub exposure_class: String,
    #[serde(default)]
    pub required_roles: Option<Vec<String>>,
    #[serde(default)]
    pub rate_profile: Option<String>,
}

/// Backend route policy.
#[derive(Debug, Deserialize)]
pub struct BackendRoutePolicy {
    pub path: String,
    pub methods: Vec<String>,
    #[serde(default)]
    pub required_roles: Option<Vec<String>>,
    #[serde(default)]
    pub rate_profile: Option<String>,
}

/// Service-to-service call policy.
#[derive(Debug, Deserialize)]
pub struct ServiceCallPolicy {
    pub id: String,
    pub source_service: String,
    pub target_service: String,
    pub target_auth_policy: ServiceTargetAuthPolicy,
    #[serde(default)]
    pub current_auth_policy: Option<ServiceCurrentAuthPolicy>,
}

/// Target auth policy block for a service call.
#[derive(Debug, Deserialize)]
pub struct ServiceTargetAuthPolicy {
    pub method: String,
    pub service_identity: String,
}

/// Current auth policy block for a service call.
#[derive(Debug, Deserialize)]
pub struct ServiceCurrentAuthPolicy {
    #[serde(default)]
    pub env: Option<String>,
}

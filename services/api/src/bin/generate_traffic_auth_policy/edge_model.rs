//! Serializable projection models for the edge policy JSON outputs.
//!
//! serde serializes struct fields in declaration order, so field order here
//! reproduces the `[ordered]@{}` blocks from the retired
//! `phase-05-edge-projection.ps1` and `phase-06-aws-waf-manifest.ps1`. Biome
//! reformats whitespace afterwards but never reorders keys.

use serde::Serialize;

/// Provider-neutral edge ingress projection root.
#[derive(Debug, Serialize)]
pub struct EdgeProjection {
    pub schema_version: &'static str,
    pub source_registry: &'static str,
    pub projection_kind: &'static str,
    pub generated_targets: Vec<&'static str>,
    pub public_route_rules: Vec<PublicEdgeRule>,
    pub auth_route_rules: Vec<AuthEdgeRule>,
    pub api_proxy_route_rules: Vec<ApiProxyEdgeRule>,
    pub service_to_service_rules: Vec<ServiceEdgeRule>,
}

/// Projected rate block shared across edge rules.
#[derive(Debug, Serialize, Clone)]
pub struct RateProjection {
    pub key_strategy: String,
    pub key_prefix: String,
    pub limit: i64,
    pub window_seconds: i64,
    pub problem_type: String,
}

/// Public route edge rule.
#[derive(Debug, Serialize)]
pub struct PublicEdgeRule {
    pub source_policy_id: String,
    pub proxy_path: String,
    pub backend_route: String,
    pub methods: Vec<String>,
    pub exposure_class: &'static str,
    pub rate: RateProjection,
    pub forbidden_request_shapes: Vec<String>,
}

/// Auth route edge rule.
#[derive(Debug, Serialize)]
pub struct AuthEdgeRule {
    pub source_policy_id: String,
    pub path_source: String,
    pub methods: Vec<String>,
    pub rate: RateProjection,
}

/// API proxy route edge rule.
#[derive(Debug, Serialize)]
pub struct ApiProxyEdgeRule {
    pub source_policy_id: String,
    pub edge_path: String,
    pub target_path: String,
    pub target_path_kind: String,
    pub methods: Vec<String>,
    pub exposure_class: String,
    pub required_roles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate: Option<RateProjection>,
}

/// Service-to-service edge rule.
#[derive(Debug, Serialize)]
pub struct ServiceEdgeRule {
    pub source_policy_id: String,
    pub source_service: String,
    pub target_service: String,
    pub target_auth_method: String,
    pub service_identity: String,
    pub current_auth_env: Option<String>,
}

/// AWS `WAFv2` manifest root.
#[derive(Debug, Serialize)]
pub struct AwsWafManifest {
    pub schema_version: &'static str,
    pub source_projection: &'static str,
    pub source_registry: &'static str,
    pub managed_by: &'static str,
    pub scope_options: Vec<&'static str>,
    pub rate_based_rules: Vec<AwsWafRateRule>,
    pub blocked_query_shape_rules: Vec<BlockedQueryShapeRule>,
    pub identity_aware_application_rules: Vec<IdentityAwareApplicationRule>,
    pub service_identity_rules: Vec<ServiceIdentityRule>,
}

/// AWS `WAFv2` rate-based rule.
#[derive(Debug, Serialize)]
pub struct AwsWafRateRule {
    pub source_policy_id: String,
    pub priority: i64,
    pub aggregate_key_type: &'static str,
    pub limit_per_5m: i64,
    pub r#match: AwsWafRateMatch,
}

/// Match block for a rate-based rule.
#[derive(Debug, Serialize)]
pub struct AwsWafRateMatch {
    pub path_match: String,
    pub methods: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_source: Option<String>,
}

/// AWS `WAFv2` blocked query shape rule.
#[derive(Debug, Serialize)]
pub struct BlockedQueryShapeRule {
    pub source_policy_id: String,
    pub priority: i64,
    pub action: &'static str,
    pub r#match: BlockedQueryShapeMatch,
}

/// Match block for a blocked query shape rule.
#[derive(Debug, Serialize)]
pub struct BlockedQueryShapeMatch {
    pub path: String,
    pub path_match: String,
    pub query_parameters: Vec<String>,
}

/// AWS `WAFv2` identity-aware application rule.
#[derive(Debug, Serialize)]
pub struct IdentityAwareApplicationRule {
    pub source_policy_id: String,
    pub reason: &'static str,
}

/// AWS `WAFv2` service identity rule.
#[derive(Debug, Serialize)]
pub struct ServiceIdentityRule {
    pub source_policy_id: String,
    pub target_auth_method: String,
}

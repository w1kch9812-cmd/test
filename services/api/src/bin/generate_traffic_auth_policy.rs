//! Generates the traffic/auth policy artifacts from the registry SSOT.
//!
//! Reads `docs/architecture/traffic-auth-policy-registry.v1.json` (hand-edited
//! SSOT) and writes the six generated policy files. The two `.rs` outputs are
//! emitted verbatim; the two `.ts` and two `.json` outputs are emitted and then
//! formatted with the repo's Biome so the committed artifacts reproduce
//! byte-for-byte.
//!
//! Run with `cargo run -p api --bin generate-traffic-auth-policy`.

#![forbid(unsafe_code)]
// This binary emits source files line by line, so the line-builder style and
// long per-file render functions are intentional.
#![allow(
    clippy::disallowed_methods,
    clippy::vec_init_then_push,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[path = "generate_traffic_auth_policy/edge_model.rs"]
mod edge_model;
#[path = "generate_traffic_auth_policy/registry.rs"]
mod registry;
#[path = "generate_traffic_auth_policy/transform.rs"]
mod transform;

use edge_model::{
    ApiProxyEdgeRule, AuthEdgeRule, AwsWafManifest, AwsWafRateMatch, AwsWafRateRule,
    BlockedQueryShapeMatch, BlockedQueryShapeRule, EdgeProjection, IdentityAwareApplicationRule,
    PublicEdgeRule, RateProjection, ServiceEdgeRule, ServiceIdentityRule,
};
use registry::{PublicRoutePolicy, Registry, RouteRateProfile, EXPECTED_SCHEMA_VERSION};

/// Header comment for `.ts` outputs (line-comment style).
const TS_HEADER: &str = "// Generated from docs/architecture/traffic-auth-policy-registry.v1.json.\n// Run `cargo run -p api --bin generate-traffic-auth-policy` after editing the registry.\n";
/// Header comment for `.rs` outputs (doc-comment style).
const RS_TRAFFIC_HEADER: &str = "//! Generated traffic/auth serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.\n//! Run `cargo run -p api --bin generate-traffic-auth-policy` after editing the registry.\n";
/// Header comment for the listing marker `.rs` output.
const RS_MARKER_HEADER: &str = "//! Generated listing marker serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.\n//! Run `cargo run -p api --bin generate-traffic-auth-policy` after editing the registry.\n";

fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt::try_init();
    match run() {
        Ok(()) => {
            tracing::info!("traffic-auth-policy-generated");
            ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!(error = %error, "traffic-auth-policy generation failed");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = resolve_repo_root()?;
    let registry = load_registry(&root)?;

    // Phase 4: Rust serving policies (verbatim, no Biome).
    let traffic_rs = render_traffic_auth_policy_rs(&registry)?;
    write_file(
        &root.join("services/api/src/traffic_auth_policy.rs"),
        &traffic_rs,
    )?;
    let marker_rs = render_listing_marker_policy_rs(&registry)?;
    write_file(
        &root.join("services/api/src/listing_marker_policy.rs"),
        &marker_rs,
    )?;

    // Phase 2: web route policy TS.
    let web_policy_ts = render_web_route_policy_ts(&registry)?;
    write_file(
        &root.join("apps/web/lib/policies/traffic-auth-policy.generated.ts"),
        &web_policy_ts,
    )?;

    // Phase 3: API proxy client TS.
    let api_client_ts = render_api_proxy_client_ts(&registry)?;
    write_file(
        &root.join("apps/web/lib/api/api-proxy-client.generated.ts"),
        &api_client_ts,
    )?;

    // Phase 5: provider-neutral edge projection JSON.
    let edge_json = render_edge_projection_json(&registry)?;
    write_file(
        &root.join("infrastructure/security/traffic-auth-edge-policy.generated.json"),
        &edge_json,
    )?;

    // Phase 6: AWS WAFv2 manifest JSON.
    let waf_json = render_aws_waf_manifest_json(&registry)?;
    write_file(
        &root.join("infrastructure/security/aws-wafv2-edge-policy.generated.json"),
        &waf_json,
    )?;

    // Format the TS + JSON outputs with Biome (the `.rs` files stay verbatim).
    run_biome_format(
        &root,
        &[
            "apps/web/lib/api/api-proxy-client.generated.ts",
            "apps/web/lib/policies/traffic-auth-policy.generated.ts",
            "infrastructure/security/aws-wafv2-edge-policy.generated.json",
            "infrastructure/security/traffic-auth-edge-policy.generated.json",
        ],
    )?;

    Ok(())
}

fn load_registry(root: &Path) -> Result<Registry, String> {
    let path = root.join("docs/architecture/traffic-auth-policy-registry.v1.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read registry {}: {error}", path.display()))?;
    let registry: Registry = serde_json::from_str(&text)
        .map_err(|error| format!("failed to parse registry {}: {error}", path.display()))?;
    if registry.schema_version != EXPECTED_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported schema_version '{}'",
            registry.schema_version
        ));
    }
    Ok(registry)
}

// ----------------------------------------------------------------------------
// Phase 4: Rust serving policies
// ----------------------------------------------------------------------------

fn render_traffic_auth_policy_rs(registry: &Registry) -> Result<String, String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(RS_TRAFFIC_HEADER.trim_end_matches('\n').to_string());
    lines.push(String::new());
    lines.push("use crate::backend_authorization::BackendRolePolicy;".to_string());
    lines.push(
        "use crate::backend_rate_limit::{BackendRateKeyStrategy, BackendRatePolicy};".to_string(),
    );
    lines.push("use user_domain::entity::UserRole;".to_string());
    lines.push(String::new());
    lines.push("pub const BACKEND_RATE_POLICIES: &[BackendRatePolicy] = &[".to_string());
    for route in &registry.public_route_policies {
        let path_pattern = transform::escape_double_quoted(
            &transform::backend_policy_path_to_rust_pattern(&route.backend_route),
        );
        let key_prefix = transform::escape_double_quoted(&route.rate_policy.key_prefix);
        let limit = route.rate_policy.limit;
        let window_sec = route.rate_policy.window_seconds;
        let problem_type = transform::escape_double_quoted(&route.rate_policy.problem_type);
        for method_value in &route.methods {
            let method = transform::escape_double_quoted(method_value);
            lines.push("    BackendRatePolicy {".to_string());
            lines.push(format!("        method: \"{method}\","));
            lines.push(format!("        path_pattern: \"{path_pattern}\","));
            lines.push(format!("        key_prefix: \"{key_prefix}\","));
            lines.push("        key_strategy: BackendRateKeyStrategy::ClientIp,".to_string());
            lines.push(format!("        limit: {limit},"));
            lines.push(format!("        window_seconds: {window_sec},"));
            lines.push(format!("        problem_type: \"{problem_type}\","));
            lines.push("    },".to_string());
        }
    }
    for route in &registry.backend_route_policies {
        let Some(rate_profile_id) = route.rate_profile.as_ref() else {
            continue;
        };
        let rate_profile = find_rate_profile(&registry.route_rate_profiles, rate_profile_id)?;
        let path_pattern = transform::escape_double_quoted(&route.path);
        let key_prefix = transform::escape_double_quoted(&rate_profile.key_prefix);
        let key_strategy = transform::key_strategy_to_rust(&rate_profile.key_strategy)?;
        let limit = rate_profile.limit;
        let window_sec = rate_profile.window_seconds;
        let problem_type = transform::escape_double_quoted(&rate_profile.problem_type);
        for method_value in &route.methods {
            let method = transform::escape_double_quoted(method_value);
            lines.push("    BackendRatePolicy {".to_string());
            lines.push(format!("        method: \"{method}\","));
            lines.push(format!("        path_pattern: \"{path_pattern}\","));
            lines.push(format!("        key_prefix: \"{key_prefix}\","));
            lines.push(format!("        key_strategy: {key_strategy},"));
            lines.push(format!("        limit: {limit},"));
            lines.push(format!("        window_seconds: {window_sec},"));
            lines.push(format!("        problem_type: \"{problem_type}\","));
            lines.push("    },".to_string());
        }
    }
    lines.push("];".to_string());
    lines.push(String::new());
    lines.push("pub const BACKEND_ROLE_POLICIES: &[BackendRolePolicy] = &[".to_string());
    for route in &registry.backend_route_policies {
        let required_roles = route.required_roles.clone().unwrap_or_default();
        if required_roles.is_empty() {
            continue;
        }
        let path_pattern = transform::escape_double_quoted(&route.path);
        let role_values: Result<Vec<&'static str>, String> = required_roles
            .iter()
            .map(|role| transform::role_to_rust(role))
            .collect();
        let role_literal = format!("&[{}]", role_values?.join(", "));
        for method_value in &route.methods {
            let method = transform::escape_double_quoted(method_value);
            lines.push("    BackendRolePolicy {".to_string());
            lines.push(format!("        method: \"{method}\","));
            lines.push(format!("        path_pattern: \"{path_pattern}\","));
            lines.push(format!("        required_roles: {role_literal},"));
            lines.push("    },".to_string());
        }
    }
    lines.push("];".to_string());
    Ok(join_lines(&lines))
}

fn render_listing_marker_policy_rs(registry: &Registry) -> Result<String, String> {
    let tile = find_public_route(
        &registry.public_route_policies,
        "gongzzang.public_map.listing_marker_tile",
    )?;
    let mask = find_public_route(
        &registry.public_route_policies,
        "gongzzang.public_map.listing_marker_mask",
    )?;

    let tile_budget = tile
        .response_budget
        .as_ref()
        .ok_or_else(|| "listing marker tile route missing response_budget".to_string())?;
    let tile_cache = tile
        .cache_policy
        .as_ref()
        .ok_or_else(|| "listing marker tile route missing cache_policy".to_string())?;
    let tile_single_flight = tile
        .single_flight_policy
        .as_ref()
        .ok_or_else(|| "listing marker tile route missing single_flight_policy".to_string())?;
    let mask_budget = mask
        .response_budget
        .as_ref()
        .ok_or_else(|| "listing marker mask route missing response_budget".to_string())?;

    let max_tile_bytes = require_field(tile_budget.max_tile_bytes, "max_tile_bytes")?;
    let max_features = require_field(tile_budget.max_features, "max_features")?;
    let max_mask_ids = require_field(mask_budget.max_mask_ids, "max_mask_ids")?;
    let ttl_seconds = require_field(tile_cache.ttl_seconds, "ttl_seconds")?;
    let lock_seconds = require_field(tile_single_flight.lock_seconds, "lock_seconds")?;
    let wait_attempts = require_field(tile_single_flight.wait_attempts, "wait_attempts")?;
    let wait_milliseconds =
        require_field(tile_single_flight.wait_milliseconds, "wait_milliseconds")?;

    let mut lines: Vec<String> = Vec::new();
    lines.push(RS_MARKER_HEADER.trim_end_matches('\n').to_string());
    lines.push(String::new());
    lines.push(format!(
        "pub const MAX_LISTING_MARKER_TILE_BYTES: usize = {};",
        transform::format_number_literal(max_tile_bytes)
    ));
    lines.push(format!(
        "pub const MAX_LISTING_MARKER_TILE_FEATURES: i64 = {};",
        transform::format_number_literal(max_features)
    ));
    lines.push(format!(
        "pub const MAX_LISTING_MARKER_MASK_IDS: usize = {};",
        transform::format_number_literal(max_mask_ids)
    ));
    lines.push(format!(
        "pub const LISTING_MARKER_CACHE_TTL_SECONDS: u64 = {};",
        transform::format_number_literal(ttl_seconds)
    ));
    lines.push(format!(
        "pub const LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = {};",
        transform::format_number_literal(lock_seconds)
    ));
    lines.push(format!(
        "pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = {};",
        transform::format_number_literal(wait_attempts)
    ));
    lines.push(format!(
        "pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = {};",
        transform::format_number_literal(wait_milliseconds)
    ));
    Ok(join_lines(&lines))
}

// ----------------------------------------------------------------------------
// Phase 2: web route policy TS
// ----------------------------------------------------------------------------

fn render_web_route_policy_ts(registry: &Registry) -> Result<String, String> {
    let mut lines: Vec<String> = Vec::new();
    push_ts_header(&mut lines);
    lines.push("export type GeneratedAuthRateRoutePolicy = {".to_string());
    lines.push("  readonly pathSource: string;".to_string());
    lines.push("  readonly methods: readonly (\"GET\" | \"POST\")[];".to_string());
    lines.push("  readonly rate: {".to_string());
    lines.push("    readonly keyPrefix: string;".to_string());
    lines.push("    readonly keyStrategy: \"client_ip\" | \"session_or_anon\";".to_string());
    lines.push("    readonly limit: number;".to_string());
    lines.push("    readonly windowSec: number;".to_string());
    lines.push("    readonly problemType: string;".to_string());
    lines.push("  };".to_string());
    lines.push("};".to_string());
    lines.push(String::new());
    lines.push(
        "export const GENERATED_AUTH_RATE_ROUTE_POLICIES: readonly GeneratedAuthRateRoutePolicy[] = ["
            .to_string(),
    );
    for route in &registry.auth_route_policies {
        let path_source = transform::escape_double_quoted(&route.path_source);
        let methods = transform::string_array_to_ts(&route.methods);
        let key_prefix = transform::escape_double_quoted(&route.rate_policy.key_prefix);
        let key_strategy = transform::escape_double_quoted(&route.rate_policy.key_strategy);
        let limit = route.rate_policy.limit;
        let window_sec = route.rate_policy.window_seconds;
        let problem_type = transform::escape_double_quoted(&route.rate_policy.problem_type);
        lines.push("  {".to_string());
        lines.push(format!("    pathSource: \"{path_source}\","));
        lines.push(format!("    methods: {methods},"));
        lines.push("    rate: {".to_string());
        lines.push(format!("      keyPrefix: \"{key_prefix}\","));
        lines.push(format!("      keyStrategy: \"{key_strategy}\","));
        lines.push(format!("      limit: {limit},"));
        lines.push(format!("      windowSec: {window_sec},"));
        lines.push(format!("      problemType: \"{problem_type}\","));
        lines.push("    },".to_string());
        lines.push("  },".to_string());
    }
    lines.push("];".to_string());
    lines.push(String::new());
    lines.push("export type GeneratedPageRoutePolicy = {".to_string());
    lines.push("  readonly kind: \"exact\" | \"prefix\" | \"prefix_suffix\";".to_string());
    lines.push("  readonly path?: string;".to_string());
    lines.push("  readonly pathSource?: string;".to_string());
    lines.push("  readonly prefix?: string;".to_string());
    lines.push("  readonly prefixSource?: string;".to_string());
    lines.push("  readonly suffix?: string;".to_string());
    lines.push("  readonly requiredRoles: readonly string[];".to_string());
    lines.push("};".to_string());
    lines.push(String::new());
    lines.push(
        "export const GENERATED_PAGE_ROUTE_POLICIES: readonly GeneratedPageRoutePolicy[] = ["
            .to_string(),
    );
    for route in &registry.page_route_policies {
        let kind = transform::escape_double_quoted(&route.path_kind);
        let required_roles = transform::string_array_to_ts(&route.required_roles);
        lines.push("  {".to_string());
        lines.push(format!("    kind: \"{kind}\","));
        if let Some(path) = &route.path {
            lines.push(format!(
                "    path: \"{}\",",
                transform::escape_double_quoted(path)
            ));
        }
        if let Some(path_source) = &route.path_source {
            lines.push(format!(
                "    pathSource: \"{}\",",
                transform::escape_double_quoted(path_source)
            ));
        }
        if let Some(prefix) = &route.prefix {
            lines.push(format!(
                "    prefix: \"{}\",",
                transform::escape_double_quoted(prefix)
            ));
        }
        if let Some(prefix_source) = &route.prefix_source {
            lines.push(format!(
                "    prefixSource: \"{}\",",
                transform::escape_double_quoted(prefix_source)
            ));
        }
        if let Some(suffix) = &route.suffix {
            lines.push(format!(
                "    suffix: \"{}\",",
                transform::escape_double_quoted(suffix)
            ));
        }
        lines.push(format!("    requiredRoles: {required_roles},"));
        lines.push("  },".to_string());
    }
    lines.push("];".to_string());
    lines.push(String::new());
    lines.push("export type GeneratedPublicMapRoutePolicy = {".to_string());
    lines.push("  readonly kind: \"exact\" | \"prefix\";".to_string());
    lines.push("  readonly pathSource: string;".to_string());
    lines.push("  readonly exposure: {".to_string());
    lines.push("    readonly class: \"public_derived\";".to_string());
    lines.push("    readonly allowedDataClasses: readonly string[];".to_string());
    lines.push("    readonly rawRecordAccess: \"forbidden\";".to_string());
    lines.push("    readonly bulkExport: \"forbidden\";".to_string());
    lines.push("  };".to_string());
    lines.push("  readonly rate: {".to_string());
    lines.push("    readonly keyPrefix: string;".to_string());
    lines.push("    readonly limit: number;".to_string());
    lines.push("    readonly windowSec: number;".to_string());
    lines.push("  };".to_string());
    lines.push("};".to_string());
    lines.push(String::new());
    lines.push(
        "export const GENERATED_PUBLIC_MAP_ROUTE_POLICIES: readonly GeneratedPublicMapRoutePolicy[] = ["
            .to_string(),
    );
    for route in &registry.public_route_policies {
        let kind = route.proxy_path_kind.clone();
        let path_source = transform::escape_double_quoted(&route.proxy_path_source);
        let key_prefix = transform::escape_double_quoted(&route.rate_policy.key_prefix);
        let limit = route.rate_policy.limit;
        let window_sec = route.rate_policy.window_seconds;
        let allowed_data_classes =
            transform::string_array_to_ts(&route.data_exposure_policy.allowed_data_classes);
        lines.push("  {".to_string());
        lines.push(format!("    kind: \"{kind}\","));
        lines.push(format!("    pathSource: \"{path_source}\","));
        lines.push("    exposure: {".to_string());
        lines.push("      class: \"public_derived\",".to_string());
        lines.push(format!("      allowedDataClasses: {allowed_data_classes},"));
        lines.push("      rawRecordAccess: \"forbidden\",".to_string());
        lines.push("      bulkExport: \"forbidden\",".to_string());
        lines.push("    },".to_string());
        lines.push(format!(
            "    rate: {{ keyPrefix: \"{key_prefix}\", limit: {limit}, windowSec: {window_sec} }},"
        ));
        lines.push("  },".to_string());
    }
    lines.push("];".to_string());
    lines.push(String::new());
    lines.push("export type GeneratedApiProxyRoutePolicy = {".to_string());
    lines.push("  readonly kind: \"exact\" | \"prefix\" | \"template\";".to_string());
    lines.push("  readonly targetPath: string;".to_string());
    lines.push(
        "  readonly methods: readonly (\"GET\" | \"POST\" | \"PUT\" | \"PATCH\" | \"DELETE\")[];"
            .to_string(),
    );
    lines.push(
        "  readonly exposureClass: \"public_derived\" | \"authenticated_user\" | \"privileged\";"
            .to_string(),
    );
    lines.push("  readonly requiredRoles: readonly string[];".to_string());
    lines.push("  readonly rate?: {".to_string());
    lines.push("    readonly keyPrefix: string;".to_string());
    lines.push("    readonly keyStrategy: \"session_sub\";".to_string());
    lines.push("    readonly limit: number;".to_string());
    lines.push("    readonly windowSec: number;".to_string());
    lines.push("    readonly problemType: string;".to_string());
    lines.push("  };".to_string());
    lines.push("};".to_string());
    lines.push(String::new());
    lines.push(
        "export const GENERATED_API_PROXY_ROUTE_POLICIES: readonly GeneratedApiProxyRoutePolicy[] = ["
            .to_string(),
    );
    for route in &registry.api_proxy_route_policies {
        let kind = transform::escape_double_quoted(&route.target_path_kind);
        let target_path = transform::escape_double_quoted(&route.target_path);
        let methods = transform::string_array_to_ts(&route.methods);
        let exposure_class = transform::escape_double_quoted(&route.exposure_class);
        let required_roles =
            transform::string_array_to_ts(&route.required_roles.clone().unwrap_or_default());
        lines.push("  {".to_string());
        lines.push(format!("    kind: \"{kind}\","));
        lines.push(format!("    targetPath: \"{target_path}\","));
        lines.push(format!("    methods: {methods},"));
        lines.push(format!("    exposureClass: \"{exposure_class}\","));
        lines.push(format!("    requiredRoles: {required_roles},"));
        if let Some(rate_profile_id) = &route.rate_profile {
            let rate_profile = find_rate_profile(&registry.route_rate_profiles, rate_profile_id)?;
            let key_prefix = transform::escape_double_quoted(&rate_profile.key_prefix);
            let key_strategy = transform::escape_double_quoted(&rate_profile.key_strategy);
            let limit = rate_profile.limit;
            let window_sec = rate_profile.window_seconds;
            let problem_type = transform::escape_double_quoted(&rate_profile.problem_type);
            lines.push("    rate: {".to_string());
            lines.push(format!("      keyPrefix: \"{key_prefix}\","));
            lines.push(format!("      keyStrategy: \"{key_strategy}\","));
            lines.push(format!("      limit: {limit},"));
            lines.push(format!("      windowSec: {window_sec},"));
            lines.push(format!("      problemType: \"{problem_type}\","));
            lines.push("    },".to_string());
        }
        lines.push("  },".to_string());
    }
    lines.push("];".to_string());
    Ok(join_lines(&lines))
}

// ----------------------------------------------------------------------------
// Phase 3: API proxy client TS
// ----------------------------------------------------------------------------

fn render_api_proxy_client_ts(registry: &Registry) -> Result<String, String> {
    let mut lines: Vec<String> = Vec::new();
    push_ts_header(&mut lines);
    lines.push("import type { Options as KyOptions } from \"ky\";".to_string());
    lines.push("import { api } from \"@/lib/api\";".to_string());
    lines.push(String::new());
    lines.push(
        "export type ApiProxyRequestOptions = Omit<KyOptions, \"prefixUrl\" | \"method\">;"
            .to_string(),
    );
    lines.push(
        "export type ApiProxyJsonRequestOptions = Omit<KyOptions, \"prefixUrl\" | \"method\" | \"body\" | \"json\"> & {"
            .to_string(),
    );
    lines.push("  readonly json?: unknown;".to_string());
    lines.push("};".to_string());
    lines.push(String::new());
    lines.push("function encodePathParam(value: string): string {".to_string());
    lines.push("  return encodeURIComponent(value);".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
    lines.push(
        "function toJsonRequestOptions(options?: ApiProxyJsonRequestOptions): KyOptions | undefined {"
            .to_string(),
    );
    lines.push("  if (options === undefined) {".to_string());
    lines.push("    return undefined;".to_string());
    lines.push("  }".to_string());
    lines.push("  const { json, ...rest } = options;".to_string());
    lines.push("  if (json === undefined) {".to_string());
    lines.push("    return rest;".to_string());
    lines.push("  }".to_string());
    lines.push("  return { ...rest, json };".to_string());
    lines.push("}".to_string());
    lines.push(String::new());
    lines.push("export const API_PROXY_CLIENT_OPERATIONS = {".to_string());
    for route in &registry.api_proxy_route_policies {
        let operation_name = transform::policy_id_to_operation_name(&route.id)?;
        let target_path = transform::escape_double_quoted(&route.target_path);
        let methods = transform::string_array_to_ts(&route.methods);
        let source_policy_id = transform::escape_double_quoted(&route.id);
        lines.push(format!("  {operation_name}: {{"));
        lines.push(format!("    sourcePolicyId: \"{source_policy_id}\","));
        lines.push(format!("    targetPath: \"{target_path}\","));
        lines.push(format!("    methods: {methods},"));
        lines.push("  },".to_string());
    }
    lines.push("} as const;".to_string());
    lines.push(String::new());
    lines.push("export const apiProxyClient = {".to_string());
    for route in &registry.api_proxy_route_policies {
        let operation_name = transform::policy_id_to_operation_name(&route.id)?;
        let target_path = &route.target_path;
        let params = transform::api_proxy_path_parameter_names(target_path)?;
        let params_type = transform::api_proxy_params_type(&params);
        let path_expression = transform::api_proxy_target_path_to_ts_expression(target_path)?;
        lines.push(format!("  {operation_name}: {{"));
        for method_value in &route.methods {
            let method = method_value.as_str();
            let method_name = transform::request_method_name(method)?;
            if method == "GET" || method == "DELETE" {
                let signature = if params.is_empty() {
                    "options?: ApiProxyRequestOptions".to_string()
                } else {
                    format!("params: {params_type}, options?: ApiProxyRequestOptions")
                };
                lines.push(format!(
                    "    {method_name}: ({signature}) => api.{method_name}({path_expression}, options),"
                ));
                lines.push(format!(
                    "    {method_name}Json: <T>({signature}) => api.{method_name}({path_expression}, options).json<T>(),"
                ));
            } else {
                let signature = if params.is_empty() {
                    "options?: ApiProxyJsonRequestOptions".to_string()
                } else {
                    format!("params: {params_type}, options?: ApiProxyJsonRequestOptions")
                };
                lines.push(format!(
                    "    {method_name}: ({signature}) => api.{method_name}({path_expression}, toJsonRequestOptions(options)),"
                ));
                lines.push(format!(
                    "    {method_name}Json: <T>({signature}) => api.{method_name}({path_expression}, toJsonRequestOptions(options)).json<T>(),"
                ));
            }
        }
        lines.push("  },".to_string());
    }
    lines.push("} as const;".to_string());
    Ok(join_lines(&lines))
}

// ----------------------------------------------------------------------------
// Phase 5: provider-neutral edge projection JSON
// ----------------------------------------------------------------------------

fn build_api_proxy_edge_rules(registry: &Registry) -> Result<Vec<ApiProxyEdgeRule>, String> {
    let mut rules = Vec::new();
    for route in &registry.api_proxy_route_policies {
        let rate = match &route.rate_profile {
            Some(id) => {
                let profile = find_rate_profile(&registry.route_rate_profiles, id)?;
                Some(rate_projection_from_profile(profile))
            }
            None => None,
        };
        rules.push(ApiProxyEdgeRule {
            source_policy_id: route.id.clone(),
            edge_path: format!("/api/proxy/{}", route.target_path),
            target_path: route.target_path.clone(),
            target_path_kind: route.target_path_kind.clone(),
            methods: route.methods.clone(),
            exposure_class: route.exposure_class.clone(),
            required_roles: route.required_roles.clone().unwrap_or_default(),
            rate,
        });
    }
    Ok(rules)
}

fn render_edge_projection_json(registry: &Registry) -> Result<String, String> {
    let public_rules: Vec<PublicEdgeRule> = registry
        .public_route_policies
        .iter()
        .map(|route| PublicEdgeRule {
            source_policy_id: route.id.clone(),
            proxy_path: route.proxy_path.clone(),
            backend_route: route.backend_route.clone(),
            methods: route.methods.clone(),
            exposure_class: "public_derived",
            rate: RateProjection {
                key_strategy: "client_ip".to_string(),
                key_prefix: route.rate_policy.key_prefix.clone(),
                limit: route.rate_policy.limit,
                window_seconds: route.rate_policy.window_seconds,
                problem_type: route.rate_policy.problem_type.clone(),
            },
            forbidden_request_shapes: route.forbidden_request_shapes.clone().unwrap_or_default(),
        })
        .collect();

    let auth_rules: Vec<AuthEdgeRule> = registry
        .auth_route_policies
        .iter()
        .map(|route| AuthEdgeRule {
            source_policy_id: route.id.clone(),
            path_source: route.path_source.clone(),
            methods: route.methods.clone(),
            rate: RateProjection {
                key_strategy: route.rate_policy.key_strategy.clone(),
                key_prefix: route.rate_policy.key_prefix.clone(),
                limit: route.rate_policy.limit,
                window_seconds: route.rate_policy.window_seconds,
                problem_type: route.rate_policy.problem_type.clone(),
            },
        })
        .collect();

    let api_proxy_rules = build_api_proxy_edge_rules(registry)?;

    let service_rules: Vec<ServiceEdgeRule> = registry
        .service_call_policies
        .iter()
        .map(|policy| ServiceEdgeRule {
            source_policy_id: policy.id.clone(),
            source_service: policy.source_service.clone(),
            target_service: policy.target_service.clone(),
            target_auth_method: policy.target_auth_policy.method.clone(),
            service_identity: policy.target_auth_policy.service_identity.clone(),
            current_auth_env: policy
                .current_auth_policy
                .as_ref()
                .and_then(|current| current.env.clone()),
        })
        .collect();

    let projection = EdgeProjection {
        schema_version: "gongzzang.traffic_auth_edge_policy_projection.v1",
        source_registry: "docs/architecture/traffic-auth-policy-registry.v1.json",
        projection_kind: "provider_neutral_edge_ingress",
        generated_targets: vec!["cloudfront", "aws_wafv2", "alb", "service_mesh"],
        public_route_rules: public_rules,
        auth_route_rules: auth_rules,
        api_proxy_route_rules: api_proxy_rules,
        service_to_service_rules: service_rules,
    };
    serialize_json(&projection)
}

// ----------------------------------------------------------------------------
// Phase 6: AWS WAFv2 manifest JSON
// ----------------------------------------------------------------------------

fn render_aws_waf_manifest_json(registry: &Registry) -> Result<String, String> {
    let mut rate_based_rules: Vec<AwsWafRateRule> = Vec::new();
    let mut priority = 1000_i64;
    for route in &registry.public_route_policies {
        let path_match = transform::path_kind_to_aws_waf_path_match(&route.proxy_path_kind)?;
        let limit = transform::rate_to_five_minute_limit(
            route.rate_policy.limit,
            route.rate_policy.window_seconds,
            &route.rate_policy.key_prefix,
        )?;
        rate_based_rules.push(AwsWafRateRule {
            source_policy_id: route.id.clone(),
            priority,
            aggregate_key_type: "IP",
            limit_per_5m: limit,
            r#match: AwsWafRateMatch {
                path_match: path_match.to_string(),
                methods: route.methods.clone(),
                path: non_empty(&route.proxy_path),
                path_source: None,
            },
        });
        priority += 10;
    }
    for route in &registry.auth_route_policies {
        if route.rate_policy.key_strategy != "client_ip" {
            continue;
        }
        let limit = transform::rate_to_five_minute_limit(
            route.rate_policy.limit,
            route.rate_policy.window_seconds,
            &route.rate_policy.key_prefix,
        )?;
        let path = transform::resolve_auth_path_source(&route.path_source)?;
        rate_based_rules.push(AwsWafRateRule {
            source_policy_id: route.id.clone(),
            priority,
            aggregate_key_type: "IP",
            limit_per_5m: limit,
            r#match: AwsWafRateMatch {
                path_match: "EXACT".to_string(),
                methods: route.methods.clone(),
                path: non_empty(path),
                path_source: non_empty(&route.path_source),
            },
        });
        priority += 10;
    }

    let mut blocked_query_shape_rules: Vec<BlockedQueryShapeRule> = Vec::new();
    let mut blocked_priority = 2000_i64;
    for route in &registry.public_route_policies {
        let forbidden = route.forbidden_request_shapes.clone().unwrap_or_default();
        if forbidden.is_empty() {
            continue;
        }
        let path_match = transform::path_kind_to_aws_waf_path_match(&route.proxy_path_kind)?;
        blocked_query_shape_rules.push(BlockedQueryShapeRule {
            source_policy_id: route.id.clone(),
            priority: blocked_priority,
            action: "BLOCK",
            r#match: BlockedQueryShapeMatch {
                path: route.proxy_path.clone(),
                path_match: path_match.to_string(),
                query_parameters: forbidden,
            },
        });
        blocked_priority += 10;
    }

    let mut identity_aware_application_rules: Vec<IdentityAwareApplicationRule> = Vec::new();
    for route in &registry.auth_route_policies {
        if route.rate_policy.key_strategy != "client_ip" {
            identity_aware_application_rules.push(IdentityAwareApplicationRule {
                source_policy_id: route.id.clone(),
                reason: "key_strategy_not_representable_in_wafv2",
            });
        }
    }
    let api_proxy_rules = build_api_proxy_edge_rules(registry)?;
    for rule in &api_proxy_rules {
        if let Some(rate) = &rule.rate {
            if rate.key_strategy != "client_ip" {
                identity_aware_application_rules.push(IdentityAwareApplicationRule {
                    source_policy_id: rule.source_policy_id.clone(),
                    reason: "key_strategy_not_representable_in_wafv2",
                });
            }
        }
    }

    let service_identity_rules: Vec<ServiceIdentityRule> = registry
        .service_call_policies
        .iter()
        .map(|policy| ServiceIdentityRule {
            source_policy_id: policy.id.clone(),
            target_auth_method: policy.target_auth_policy.method.clone(),
        })
        .collect();

    let manifest = AwsWafManifest {
        schema_version: "gongzzang.aws_wafv2_edge_policy_manifest.v1",
        source_projection: "infrastructure/security/traffic-auth-edge-policy.generated.json",
        source_registry: "docs/architecture/traffic-auth-policy-registry.v1.json",
        managed_by: "pulumi",
        scope_options: vec!["CLOUDFRONT", "REGIONAL"],
        rate_based_rules,
        blocked_query_shape_rules,
        identity_aware_application_rules,
        service_identity_rules,
    };
    serialize_json(&manifest)
}

// ----------------------------------------------------------------------------
// Shared helpers
// ----------------------------------------------------------------------------

fn rate_projection_from_profile(profile: &RouteRateProfile) -> RateProjection {
    RateProjection {
        key_strategy: profile.key_strategy.clone(),
        key_prefix: profile.key_prefix.clone(),
        limit: profile.limit,
        window_seconds: profile.window_seconds,
        problem_type: profile.problem_type.clone(),
    }
}

fn non_empty(value: &str) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn require_field(value: Option<i64>, name: &str) -> Result<i64, String> {
    value.ok_or_else(|| format!("registry field missing: {name}"))
}

fn find_rate_profile<'a>(
    profiles: &'a [RouteRateProfile],
    id: &str,
) -> Result<&'a RouteRateProfile, String> {
    profiles
        .iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| format!("Missing API proxy rate profile id={id}"))
}

fn find_public_route<'a>(
    routes: &'a [PublicRoutePolicy],
    id: &str,
) -> Result<&'a PublicRoutePolicy, String> {
    routes
        .iter()
        .find(|route| route.id == id)
        .ok_or_else(|| format!("Missing public route policy id={id}"))
}

fn push_ts_header(lines: &mut Vec<String>) {
    for line in TS_HEADER.trim_end_matches('\n').split('\n') {
        lines.push(line.to_string());
    }
    lines.push(String::new());
}

fn serialize_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize JSON: {error}"))?;
    text.push('\n');
    Ok(text)
}

fn join_lines(lines: &[String]) -> String {
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, contents)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn run_biome_format(root: &Path, targets: &[&str]) -> Result<(), String> {
    let biome = resolve_biome_executable(root)?;
    let mut command = Command::new(&biome);
    command.current_dir(root).arg("format").arg("--write");
    for target in targets {
        command.arg(target);
    }
    let status = command
        .status()
        .map_err(|error| format!("failed to launch Biome ({}): {error}", biome.display()))?;
    if !status.success() {
        return Err(
            "Biome formatting failed for generated traffic/auth policy outputs.".to_string(),
        );
    }
    Ok(())
}

fn resolve_biome_executable(root: &Path) -> Result<PathBuf, String> {
    let candidates = [
        "node_modules/.bin/biome.CMD",
        "node_modules/.bin/biome.cmd",
        "node_modules/.bin/biome.exe",
        "node_modules/.bin/biome",
    ];
    for candidate in candidates {
        let path = root.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err("Biome executable is missing; run package install before generating traffic/auth policy outputs.".to_string())
}

fn resolve_repo_root() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let mut candidate = Some(current_dir.as_path());
    while let Some(dir) = candidate {
        if dir.join(".git").exists() || has_workspace_manifest(dir) {
            return Ok(dir.to_path_buf());
        }
        candidate = dir.parent();
    }
    Err("repo root is missing: Cargo.toml or .git".to_string())
}

fn has_workspace_manifest(dir: &Path) -> bool {
    let manifest = dir.join("Cargo.toml");
    std::fs::read_to_string(manifest)
        .is_ok_and(|content| content.lines().any(|line| line == "[workspace]"))
}

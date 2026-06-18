function Write-TrafficAuthBackendFixtures {
    Write-File -Root $Root -RelativePath "services\api\src\listing_marker_policy.rs" -Content @'
MAX_LISTING_MARKER_TILE_BYTES: usize = 262_144;
MAX_LISTING_MARKER_TILE_FEATURES: i64 = 10_000;
MAX_LISTING_MARKER_MASK_IDS: usize = 20_000;
LISTING_MARKER_CACHE_TTL_SECONDS: u64 = 30;
LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = 5;
LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = 10;
LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = 50;
'@
    $generatedBackendRolePolicies = if ($OmitGeneratedBackendRolePolicies) {
        ""
    } else {
        @'
BACKEND_ROLE_POLICIES
BackendRolePolicy {
    method: "POST",
    path_pattern: "/listings",
    required_roles: &[UserRole::Broker],
}
BackendRolePolicy {
    method: "PATCH",
    path_pattern: "/listings/:id",
    required_roles: &[UserRole::Broker],
}
'@
    }
    Write-File -Root $Root -RelativePath "services\api\src\traffic_auth_policy.rs" -Content @"
BACKEND_RATE_POLICIES
method: "GET"
method: "POST"
method: "PATCH"
path_pattern: "/listings"
path_pattern: "/listings/:listing_id/photos/:photo_id"
path_pattern: "/listings/:id"
key_prefix: "public-map:listing-marker-tile"
key_prefix: "public-map:listing-marker-count"
key_prefix: "public-map:listing-marker-filter"
key_prefix: "public-map:listing-marker-mask"
key_prefix: "public-map:listing-marker-tombstone"
key_prefix: "public-map:listing-marker-delta"
key_prefix: "api-proxy:authenticated-read"
key_prefix: "api-proxy:privileged-write"
limit: 600
limit: 120
limit: 60
limit: 240
window_seconds: 60
problem_type: "map/too-many-public-marker-requests"
problem_type: "proxy/too-many-requests"
$generatedBackendRolePolicies
"@
    Write-File -Root $Root -RelativePath "services\api\src\listing_marker_serving\mod.rs" -Content @'
crate::listing_marker_policy
'@
    $protectedAuthLayer = if ($OmitBackendProtectedAuthLayer) { "" } else { "auth_layer" }
    $backendAuthorizationModule = if ($OmitBackendAuthorizationLayer) { "" } else { "mod backend_authorization;" }
    $backendAuthorizationLayer = if ($OmitBackendAuthorizationLayer) {
        ""
    } else {
        @'
use crate::backend_authorization::{enforce_backend_roles, BackendAuthorizationState};
let backend_authorization_state = BackendAuthorizationState::new(traffic_auth_policy::BACKEND_ROLE_POLICIES);
'@
    }
    $backendAuthorizationLayerMount = if ($OmitBackendAuthorizationLayer) {
        ""
    } else {
        "    .layer(middleware::from_fn_with_state(backend_authorization_state.clone(), enforce_backend_roles))"
    }
    Write-File -Root $Root -RelativePath "services\api\src\main.rs" -Content @"
$backendAuthorizationModule
mod backend_rate_limit;
mod traffic_auth_policy;
"@
    Write-File -Root $Root -RelativePath "services\api\src\app.rs" -Content @"
$backendAuthorizationLayer
use crate::backend_rate_limit::{enforce_backend_rate_limit, RedisBackendRateLimiter};
let backend_rate_limiter = RedisBackendRateLimiter::new(redis_pool.clone());
let listing_marker_tiles_router: Router<()> = Router::new()
    .route("/map/v1/marker-tiles/listing/:z/:x/:y_pbf", get(routes::listing_marker_tiles::get_listing_marker_tile))
    .layer(middleware::from_fn_with_state(traffic_auth_policy::BACKEND_RATE_POLICIES, enforce_backend_rate_limit));
let listings_router: Router<()> = Router::new()
    .route("/listings", get(routes::listings::get_listings).post(routes::listings::create_listing))
    .route("/listings/:id", get(routes::listings::get_listing).patch(routes::listings::patch_listing))
    .route("/listings/:listing_id/photos/:photo_id", get(routes::listings::get_photo_download_redirect))
$backendAuthorizationLayerMount
    .layer(middleware::from_fn_with_state(traffic_auth_policy::BACKEND_RATE_POLICIES, enforce_backend_rate_limit))
    .layer(middleware::from_fn_with_state(auth_state.clone(), $protectedAuthLayer));
let internal_auth_secret = build_internal_auth_secret(is_production)?;
let internal: Router<()> = Router::new()
    .route("/internal/auth/event", axum::routing::post(routes::auth_event::post_auth_event))
    .with_state(auth_event_state);
"@
    if ($AddUnregisteredBackendRoute) {
        Write-File -Root $Root -RelativePath "services\api\src\app.rs" -Content @"
$backendAuthorizationLayer
use crate::backend_rate_limit::{enforce_backend_rate_limit, RedisBackendRateLimiter};
let backend_rate_limiter = RedisBackendRateLimiter::new(redis_pool.clone());
let listing_marker_tiles_router: Router<()> = Router::new()
    .route("/map/v1/marker-tiles/listing/:z/:x/:y_pbf", get(routes::listing_marker_tiles::get_listing_marker_tile))
    .layer(middleware::from_fn_with_state(traffic_auth_policy::BACKEND_RATE_POLICIES, enforce_backend_rate_limit));
let listings_router: Router<()> = Router::new()
    .route("/listings", get(routes::listings::get_listings).post(routes::listings::create_listing))
    .route("/listings/:id", get(routes::listings::get_listing).patch(routes::listings::patch_listing))
    .route("/listings/:listing_id/photos/:photo_id", get(routes::listings::get_photo_download_redirect))
    .route("/unregistered", get(routes::debug::unregistered))
$backendAuthorizationLayerMount
    .layer(middleware::from_fn_with_state(traffic_auth_policy::BACKEND_RATE_POLICIES, enforce_backend_rate_limit))
    .layer(middleware::from_fn_with_state(auth_state.clone(), $protectedAuthLayer));
let internal_auth_secret = build_internal_auth_secret(is_production)?;
let internal: Router<()> = Router::new()
    .route("/internal/auth/event", axum::routing::post(routes::auth_event::post_auth_event))
    .with_state(auth_event_state);
"@
    }
    Write-File -Root $Root -RelativePath "services\api\src\routes\health.rs" -Content @'
Router::new()
    .route("/healthz", get(liveness))
    .route("/healthz/ready", get(readiness))
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\routes.ts" -Content @'
export const API = {
  auth: {
    login: "/api/auth/login",
    callback: "/api/auth/callback",
    refresh: "/api/auth/refresh",
    logout: "/api/auth/logout",
  },
  proxy: {
    listingMarkerTilesPrefix: `${API_PROXY_BASE}/map/v1/marker-tiles/listing`,
  },
};

const API_PROXY_BASE = "/api/proxy";
'@
    Write-File -Root $Root -RelativePath "apps\web\app\api\auth\logout\route.ts" -Content @'
export function POST() {}
export function GET() {}
'@
    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-boundary.v1.json" -Content @'
PLATFORM_CORE_SERVICE_TOKEN
PLATFORM_CORE_WEBHOOK_SECRET
direct_platform_core_database
'@
}

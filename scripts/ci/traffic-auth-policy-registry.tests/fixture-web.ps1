function Write-TrafficAuthWebFixtures {
    Write-File -Root $Root -RelativePath "apps\web\proxy.ts" -Content @'
GENERATED_PUBLIC_MAP_ROUTE_POLICIES
GENERATED_AUTH_RATE_ROUTE_POLICIES
GENERATED_PAGE_ROUTE_POLICIES
getAuthRateRoutePolicy
resolveAuthRateKey
getPageRoutePolicy
exposure: policy.exposure
API.auth.login
API.auth.callback
API.auth.refresh
API.auth.logout
API.proxy.listingMarkerTilesPrefix
API.proxy.listingMarkerCounts
API.proxy.listingMarkerFilters
API.proxy.listingMarkerTombstonesPrefix
API.proxy.listingMarkerDeltasPrefix
LISTING_MARKER_MASK_PREFIX
'@
    $apiProxyExposureGate = if ($OmitApiProxyExposureGate) {
        ""
    } else {
        @'
enforceApiProxyExposure
sessionRequiredProblem
insufficientRoleProblem
policy.requiredRoles.includes
'@
    }
    Write-File -Root $Root -RelativePath "apps\web\app\api\proxy\[...path]\route.ts" -Content @"
GENERATED_API_PROXY_ROUTE_POLICIES
getApiProxyRoutePolicy
proxy/route-not-allowed
checkApiProxyRateLimit
resolveApiProxyRateKey
checkRate(
$apiProxyExposureGate
"@
    if ($AddUnregisteredApiProxyClientUsage) {
        Write-File -Root $Root -RelativePath "apps\web\lib\unregistered-proxy-client.ts" -Content @'
export const unregisteredProxyTarget = "/api/proxy/me/notifications";
'@
    }
    $generatedExposureMetadata = if ($OmitGeneratedExposureMetadata) {
        ""
    } else {
        @'
class: "public_derived"
rawRecordAccess: "forbidden"
bulkExport: "forbidden"
allowedDataClasses: ["derived_marker_tile"]
allowedDataClasses: ["aggregate_count"]
allowedDataClasses: ["opaque_filter_hash"]
allowedDataClasses: ["marker_id_mask"]
'@
    }
    $generatedAuthRatePolicies = if ($OmitGeneratedAuthRatePolicies) {
        ""
    } else {
        $generatedAuthLogoutRatePolicy = if ($OmitAuthLogoutPolicy) {
            ""
        } else {
            @'
API.auth.logout
auth:logout
limit: 30
'@
        }
        @'
GENERATED_AUTH_RATE_ROUTE_POLICIES
API.auth.login
API.auth.callback
API.auth.refresh
auth:login
auth:callback
auth:refresh
keyStrategy: "client_ip"
keyStrategy: "session_or_anon"
limit: 5
limit: 10
limit: 30
windowSec: 60
problemType: "auth/too-many-requests"
'@ + "`n$generatedAuthLogoutRatePolicy"
    }
    $generatedPageRoutePolicies = if ($OmitGeneratedPageRoutePolicies) {
        ""
    } else {
        $generatedListingPageRoles = if ($AllowAdminListingPageRole) { 'requiredRoles: ["Broker", "Admin"]' } else { 'requiredRoles: ["Broker"]' }
        @'
GENERATED_PAGE_ROUTE_POLICIES
kind: "prefix"
kind: "exact"
kind: "prefix_suffix"
path: "/admin"
pathSource: "ROUTES.listings.new"
prefixSource: "ROUTES.listings.index"
suffix: "/edit"
requiredRoles: ["Admin", "Broker", "Operator"]
'@ + "`n$generatedListingPageRoles`n"
    }
    Write-File -Root $Root -RelativePath "apps\web\lib\policies\traffic-auth-policy.generated.ts" -Content @"
$generatedAuthRatePolicies
$generatedPageRoutePolicies
GENERATED_API_PROXY_ROUTE_POLICIES
keyPrefix: "api-proxy:authenticated-read"
keyPrefix: "api-proxy:authenticated-write"
keyPrefix: "api-proxy:privileged-write"
keyStrategy: "session_sub"
limit: 240
limit: 120
limit: 60
windowSec: 60
problemType: "proxy/too-many-requests"
map/v1/marker-tiles/listing/:z/:x/:y_pbf
listings/:listing_id/photos/:photo_id
listings
listings/:id
requiredRoles: []
requiredRoles: ["Broker"]
API.proxy.listingMarkerTilesPrefix
public-map:listing-marker-tile
limit: 600
windowSec: 60
API.proxy.listingMarkerCounts
public-map:listing-marker-count
limit: 120
windowSec: 60
API.proxy.listingMarkerFilters
public-map:listing-marker-filter
limit: 60
windowSec: 60
LISTING_MARKER_MASK_PREFIX
public-map:listing-marker-mask
limit: 120
windowSec: 60
API.proxy.listingMarkerTombstonesPrefix
public-map:listing-marker-tombstone
limit: 120
windowSec: 60
API.proxy.listingMarkerDeltasPrefix
public-map:listing-marker-delta
limit: 120
windowSec: 60
$generatedExposureMetadata
"@
}

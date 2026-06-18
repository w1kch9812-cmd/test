    Write-File -Root $Root -RelativePath "apps\web\lib\identity\patterns.ts" -Content @'
PNU_PATTERN
LISTING_ID_PATTERN
lst_[0-9A-HJKMNP-TV-Z]{26}
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\marker-tile-contract.ts" -Content @'
LISTING_MARKER_TILE_LAYER
LISTING_MARKER_DELTA_TILE_LAYER
LISTING_MARKER_TILE_ENDPOINT_TEMPLATE
buildListingMarkerDeltaTileSource
buildListingMarkerTombstoneUrl
createListingMarkerOverlayState
ALL_ACTIVE_MARKER_FILTER_HASH
buildListingMarkerTileSource
assertSupportedListingFilterHash
resolveSameOrigin
browser origin is required for listing marker tile URLs
lst_filter_v1_[0-9a-f]{64}
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\vector-tile-manifest.ts" -Content @'
PARCEL_ANCHOR_AGGREGATE_VECTOR_TILE_LAYER
PARCEL_ANCHOR_VECTOR_TILE_LAYER
render_min_zoom
render_max_zoom
tiles_url_template
fetchVectorTileManifest
buildVectorTileSource
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\map-zoom-policy.ts" -Content @'
GONGZZANG_MAP_ZOOM_POLICY
exactParcelAnchorMinZoom: 12
parcel
minZoom: 14
maxZoom: 22
LISTING_MARKER_RENDER_MIN_ZOOM
LISTING_MARKER_RENDER_MAX_ZOOM
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\marker-tile-style.ts" -Content @'
buildParcelAnchorMarkerLayerRegistration
buildListingMarkerLayerRegistration
buildListingMarkerDeltaLayerRegistration
PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_SOURCE_ID
LISTING_MARKER_DELTA_TILE_SOURCE_ID
"source-layer": LISTING_MARKER_TILE_LAYER
'@
    Write-File -Root $Root -RelativePath "apps\web\components\listings\listing-map.tsx" -Content @'
setupMapboxRuntime
buildListingMarkerLayerFilter
buildListingMarkerServerKey
loadListingMarkerServerState
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
pushPanel({ kind: "listing", id: listingId, view: "summary" })
pushPanel({ kind: "parcel", id: pnu, view: "summary" })
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\listing-map-runtime.ts" -Content @'
setupListingMarkerTileLayers
buildListingMarkerLayerRegistration
buildListingMarkerDeltaLayerRegistration
LISTING_MARKER_RENDER_MIN_ZOOM
LISTING_MARKER_RENDER_MAX_ZOOM
buildParcelAnchorMarkerLayerRegistrations
fetchVectorTileManifest
setupMarkerTileLayers
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\routes.ts" -Content @'
listingMarkerCounts
listingMarkerFilters
listingMarkerDeltasPrefix
listingMarkerDeltaTemplate
listingMarkerMaskTemplate
listingMarkerTombstonesPrefix
listingMarkerTombstoneTemplate
marker-counts/listing
marker-filters/listing
marker-masks/listing
marker-deltas/listing
marker-tombstones/listing
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\listing-marker-filter.ts" -Content @'
buildListingMarkerLayerFilter
listing_type
transaction_type
price_krw
area_m2
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\map\listing-marker-server-state.ts" -Content @'
buildListingMarkerFilterRequest
buildListingMarkerServerKey
min_area_m2
max_price_krw
'@
    Write-File -Root $Root -RelativePath "apps\web\app\api\proxy\[...path]\route.ts" -Content @'
isBinaryProxyResponse
application/vnd.mapbox-vector-tile
arrayBuffer()
text()
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\api-proxy-route.test.ts" -Content @'
preserves Mapbox vector tile responses as binary
application/vnd.mapbox-vector-tile
arrayBuffer()
map/v1/marker-tiles/listing/14/8780/6345.pbf
'@
    Write-File -Root $Root -RelativePath "apps\web\proxy.ts" -Content @'
API.proxy.listingMarkerTilesPrefix
isLocalHostname
allowLocalHttpMapRuntime
PUBLIC_PATHS
isPublic
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\platform-core-proxy.test.ts" -Content @'
allows Gongzzang listing PBF marker tile proxy without sid
/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1
allows Naver HTTP resources only for local production preview CSP
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\panel\codec.ts" -Content @'
LISTING_ID_PATTERN
PNU_PATTERN
IdPatternViolation
'@
    Write-File -Root $Root -RelativePath "apps\web\components\panels\listing\register.ts" -Content @'
LISTING_ID_PATTERN
idPattern: LISTING_ID_PATTERN
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\listings\schema.ts" -Content @'
LISTING_ID_PATTERN
id: z.string().regex(LISTING_ID_PATTERN)
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\map\marker-tile-contract.test.ts" -Content @'
builds the Gongzzang-owned listing marker vector source through same-origin proxy
LISTING_MARKER_TILE_LAYER
LISTING_MARKER_DELTA_TILE_LAYER
http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
http://localhost:3900/api/proxy/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version=41
http://localhost:3900/api/proxy/map/v1/marker-tombstones/listing/14/13970/6344?base_version=41
not.toContain("bbox=")
not.toContain("bounds=")
'@
    Write-File -Root $Root -RelativePath "apps\web\tests\unit\map\marker-tile-style.test.ts" -Content @'
registers Gongzzang listing marker source and circle layer without coordinate inputs
registers Gongzzang listing marker delta source with the listing delta layer
LISTING_MARKER_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID
LISTING_MARKER_TILE_SOURCE_ID
LISTING_MARKER_DELTA_TILE_SOURCE_ID
http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1
'@
    Write-File -Root $Root -RelativePath "apps\web\lib\panel\codec.test.ts" -Content @'
lst_01HXY3NK0Z9F6S1B2C3D4E5F6G
rejects UUID listing ids because Listing ids are lst-prefixed ULIDs
'@

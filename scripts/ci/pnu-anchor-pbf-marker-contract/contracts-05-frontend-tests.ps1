$contracts += @(
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/map-zoom-policy.ts"
        Tokens = @(
            "GONGZZANG_MAP_ZOOM_POLICY",
            "exactParcelAnchorMinZoom: 12",
            "parcel",
            "minZoom: 14",
            "maxZoom: 22",
            "LISTING_MARKER_RENDER_MIN_ZOOM",
            "LISTING_MARKER_RENDER_MAX_ZOOM"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/marker-tile-style.ts"
        Tokens = @(
            "buildParcelAnchorMarkerLayerRegistration",
            "buildListingMarkerLayerRegistration",
            "buildListingMarkerDeltaLayerRegistration",
            "PARCEL_ANCHOR_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_SOURCE_ID",
            "LISTING_MARKER_DELTA_TILE_SOURCE_ID",
            '"source-layer": LISTING_MARKER_TILE_LAYER'
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "lat=",
            "lng="
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/components/listings/listing-map.tsx"
        Tokens = @(
            "setupMapboxRuntime",
            "buildListingMarkerLayerFilter",
            "buildListingMarkerServerKey",
            "loadListingMarkerServerState",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            'pushPanel({ kind: "listing", id: listingId, view: "summary" })',
            'pushPanel({ kind: "parcel", id: pnu, view: "summary" })'
        )
        Forbidden = @(
            "new naver.maps.Marker",
            "listing.lat",
            "listing.lng",
            "pinIconHtml",
            "markersRef",
            "bounds_changed",
            "boundsTimerRef",
            "setBounds",
            "getBounds()"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/listing-map-runtime.ts"
        Tokens = @(
            "setupListingMarkerTileLayers",
            "buildListingMarkerLayerRegistration",
            "buildListingMarkerDeltaLayerRegistration",
            "LISTING_MARKER_RENDER_MIN_ZOOM",
            "LISTING_MARKER_RENDER_MAX_ZOOM",
            "buildParcelAnchorMarkerLayerRegistrations",
            "fetchVectorTileManifest",
            "setupMarkerTileLayers"
        )
        Forbidden = @(
            "bounds=",
            "bbox=",
            "lat=",
            "lng="
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/routes.ts"
        Tokens = @(
            "listingMarkerCounts",
            "listingMarkerFilters",
            "listingMarkerDeltasPrefix",
            "listingMarkerDeltaTemplate",
            "listingMarkerMaskTemplate",
            "listingMarkerTombstonesPrefix",
            "listingMarkerTombstoneTemplate",
            "marker-counts/listing",
            "marker-filters/listing",
            "marker-masks/listing",
            "marker-deltas/listing",
            "marker-tombstones/listing"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/listing-marker-filter.ts"
        Tokens = @(
            "buildListingMarkerLayerFilter",
            "listing_type",
            "transaction_type",
            "price_krw",
            "area_m2"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/map/listing-marker-server-state.ts"
        Tokens = @(
            "buildListingMarkerFilterRequest",
            "buildListingMarkerServerKey",
            "min_area_m2",
            "max_price_krw"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/app/api/proxy/[...path]/route.ts"
        Tokens = @(
            "isBinaryProxyResponse",
            "application/vnd.mapbox-vector-tile",
            "arrayBuffer()",
            "text()"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/api-proxy-route.test.ts"
        Tokens = @(
            "preserves Mapbox vector tile responses as binary",
            "application/vnd.mapbox-vector-tile",
            "arrayBuffer()",
            "map/v1/marker-tiles/listing/14/8780/6345.pbf"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/proxy.ts"
        Tokens = @(
            "API.proxy.listingMarkerTilesPrefix",
            "isLocalHostname",
            "allowLocalHttpMapRuntime",
            "PUBLIC_PATHS",
            "isPublic"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/platform-core-proxy.test.ts"
        Tokens = @(
            "allows Gongzzang listing PBF marker tile proxy without sid",
            "/api/proxy/map/v1/marker-tiles/listing/14/8780/6345.pbf?filter_hash=all-active-v1",
            "allows Naver HTTP resources only for local production preview CSP"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/panel/codec.ts"
        Tokens = @(
            "LISTING_ID_PATTERN",
            "PNU_PATTERN",
            "IdPatternViolation"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/components/panels/listing/register.ts"
        Tokens = @(
            "LISTING_ID_PATTERN",
            "idPattern: LISTING_ID_PATTERN"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/listings/schema.ts"
        Tokens = @(
            "LISTING_ID_PATTERN",
            "id: z.string().regex(LISTING_ID_PATTERN)"
        )
        Forbidden = @(
            "geom_point",
            "lat: z.number",
            "lng: z.number"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/map/marker-tile-contract.test.ts"
        Tokens = @(
            "builds the Gongzzang-owned listing marker vector source through same-origin proxy",
            "LISTING_MARKER_TILE_LAYER",
            "LISTING_MARKER_DELTA_TILE_LAYER",
            "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1",
            "http://localhost:3900/api/proxy/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version=41",
            "http://localhost:3900/api/proxy/map/v1/marker-tombstones/listing/14/13970/6344?base_version=41",
            "not.toContain(`"bbox=`")",
            "not.toContain(`"bounds=`")"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/tests/unit/map/marker-tile-style.test.ts"
        Tokens = @(
            "registers Gongzzang listing marker source and circle layer without coordinate inputs",
            "registers Gongzzang listing marker delta source with the listing delta layer",
            "LISTING_MARKER_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_DELTA_TILE_CIRCLE_LAYER_ID",
            "LISTING_MARKER_TILE_SOURCE_ID",
            "LISTING_MARKER_DELTA_TILE_SOURCE_ID",
            "http://localhost:3900/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1"
        )
    },
    [pscustomobject]@{
        RelativePath = "apps/web/lib/panel/codec.test.ts"
        Tokens = @(
            "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G",
            "rejects UUID listing ids because Listing ids are lst-prefixed ULIDs"
        )
    },
    [pscustomobject]@{
        RelativePath = "tests/migrations/test_v001_full.sh"
        Tokens = @(
            "parcel_marker_anchor",
            "parcel_marker_anchor_srid_chk",
            "parcel_marker_anchor_point_gist_idx",
            "must not duplicate anchor_lng/anchor_lat columns",
            "listing_marker_projection",
            "listing_marker_filter_registry",
            "listing_marker_projection_anchor_srid_chk",
            "listing_marker_filter_registry_spec_shape_chk",
            "platform_core_event_inbox",
            "platform_core_event_inbox_anchor_payload_chk",
            "platform_core_event_inbox_pending_idx"
        )
        Forbidden = @(
            "listing.geom_point SRID expected 4326",
            "f_geometry_column='geom_point'"
        )
    },
    [pscustomobject]@{
        RelativePath = "docs/frontend/listings-search.md"
        Tokens = @(
            "Listing Marker Serving",
            "listing_marker_projection",
            "browser instant filter",
            "server marker indexes"
        )
    }
)

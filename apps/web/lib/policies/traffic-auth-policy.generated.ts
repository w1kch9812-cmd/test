// Generated from docs/architecture/traffic-auth-policy-registry.v1.json.
// Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.

export type GeneratedAuthRateRoutePolicy = {
  readonly pathSource: string;
  readonly methods: readonly ("GET" | "POST")[];
  readonly rate: {
    readonly keyPrefix: string;
    readonly keyStrategy: "client_ip" | "session_or_anon";
    readonly limit: number;
    readonly windowSec: number;
    readonly problemType: string;
  };
};

export const GENERATED_AUTH_RATE_ROUTE_POLICIES: readonly GeneratedAuthRateRoutePolicy[] = [
  {
    pathSource: "API.auth.login",
    methods: ["POST"],
    rate: {
      keyPrefix: "auth:login",
      keyStrategy: "client_ip",
      limit: 5,
      windowSec: 60,
      problemType: "auth/too-many-requests",
    },
  },
  {
    pathSource: "API.auth.callback",
    methods: ["GET"],
    rate: {
      keyPrefix: "auth:callback",
      keyStrategy: "client_ip",
      limit: 10,
      windowSec: 60,
      problemType: "auth/too-many-requests",
    },
  },
  {
    pathSource: "API.auth.refresh",
    methods: ["POST"],
    rate: {
      keyPrefix: "auth:refresh",
      keyStrategy: "session_or_anon",
      limit: 30,
      windowSec: 60,
      problemType: "auth/too-many-requests",
    },
  },
  {
    pathSource: "API.auth.logout",
    methods: ["POST", "GET"],
    rate: {
      keyPrefix: "auth:logout",
      keyStrategy: "client_ip",
      limit: 30,
      windowSec: 60,
      problemType: "auth/too-many-requests",
    },
  },
];

export type GeneratedPageRoutePolicy = {
  readonly kind: "exact" | "prefix" | "prefix_suffix";
  readonly path?: string;
  readonly pathSource?: string;
  readonly prefix?: string;
  readonly prefixSource?: string;
  readonly suffix?: string;
  readonly requiredRoles: readonly string[];
};

export const GENERATED_PAGE_ROUTE_POLICIES: readonly GeneratedPageRoutePolicy[] = [
  {
    kind: "prefix",
    path: "/admin",
    requiredRoles: ["Admin", "Broker", "Operator"],
  },
  {
    kind: "exact",
    pathSource: "ROUTES.listings.new",
    requiredRoles: ["Broker"],
  },
  {
    kind: "prefix_suffix",
    prefixSource: "ROUTES.listings.index",
    suffix: "/edit",
    requiredRoles: ["Broker"],
  },
];

export type GeneratedPublicMapRoutePolicy = {
  readonly kind: "exact" | "prefix";
  readonly pathSource: string;
  readonly exposure: {
    readonly class: "public_derived";
    readonly allowedDataClasses: readonly string[];
    readonly rawRecordAccess: "forbidden";
    readonly bulkExport: "forbidden";
  };
  readonly rate: {
    readonly keyPrefix: string;
    readonly limit: number;
    readonly windowSec: number;
  };
};

export const GENERATED_PUBLIC_MAP_ROUTE_POLICIES: readonly GeneratedPublicMapRoutePolicy[] = [
  {
    kind: "prefix",
    pathSource: "API.proxy.listingMarkerTilesPrefix",
    exposure: {
      class: "public_derived",
      allowedDataClasses: ["derived_marker_tile"],
      rawRecordAccess: "forbidden",
      bulkExport: "forbidden",
    },
    rate: { keyPrefix: "public-map:listing-marker-tile", limit: 600, windowSec: 60 },
  },
  {
    kind: "exact",
    pathSource: "API.proxy.listingMarkerCounts",
    exposure: {
      class: "public_derived",
      allowedDataClasses: ["aggregate_count"],
      rawRecordAccess: "forbidden",
      bulkExport: "forbidden",
    },
    rate: { keyPrefix: "public-map:listing-marker-count", limit: 120, windowSec: 60 },
  },
  {
    kind: "exact",
    pathSource: "API.proxy.listingMarkerFilters",
    exposure: {
      class: "public_derived",
      allowedDataClasses: ["opaque_filter_hash"],
      rawRecordAccess: "forbidden",
      bulkExport: "forbidden",
    },
    rate: { keyPrefix: "public-map:listing-marker-filter", limit: 60, windowSec: 60 },
  },
  {
    kind: "prefix",
    pathSource: "LISTING_MARKER_MASK_PREFIX",
    exposure: {
      class: "public_derived",
      allowedDataClasses: ["marker_id_mask"],
      rawRecordAccess: "forbidden",
      bulkExport: "forbidden",
    },
    rate: { keyPrefix: "public-map:listing-marker-mask", limit: 120, windowSec: 60 },
  },
  {
    kind: "prefix",
    pathSource: "API.proxy.listingMarkerTombstonesPrefix",
    exposure: {
      class: "public_derived",
      allowedDataClasses: ["marker_id_mask"],
      rawRecordAccess: "forbidden",
      bulkExport: "forbidden",
    },
    rate: { keyPrefix: "public-map:listing-marker-tombstone", limit: 120, windowSec: 60 },
  },
  {
    kind: "prefix",
    pathSource: "API.proxy.listingMarkerDeltasPrefix",
    exposure: {
      class: "public_derived",
      allowedDataClasses: ["derived_marker_tile"],
      rawRecordAccess: "forbidden",
      bulkExport: "forbidden",
    },
    rate: { keyPrefix: "public-map:listing-marker-delta", limit: 120, windowSec: 60 },
  },
];

export type GeneratedApiProxyRoutePolicy = {
  readonly kind: "exact" | "prefix" | "template";
  readonly targetPath: string;
  readonly methods: readonly ("GET" | "POST" | "PUT" | "PATCH" | "DELETE")[];
  readonly exposureClass: "public_derived" | "authenticated_user" | "privileged";
  readonly requiredRoles: readonly string[];
  readonly rate?: {
    readonly keyPrefix: string;
    readonly keyStrategy: "session_sub";
    readonly limit: number;
    readonly windowSec: number;
    readonly problemType: string;
  };
};

export const GENERATED_API_PROXY_ROUTE_POLICIES: readonly GeneratedApiProxyRoutePolicy[] = [
  {
    kind: "template",
    targetPath: "map/v1/marker-tiles/listing/:z/:x/:y_pbf",
    methods: ["GET"],
    exposureClass: "public_derived",
    requiredRoles: [],
  },
  {
    kind: "exact",
    targetPath: "map/v1/marker-counts/listing",
    methods: ["GET"],
    exposureClass: "public_derived",
    requiredRoles: [],
  },
  {
    kind: "exact",
    targetPath: "map/v1/marker-filters/listing",
    methods: ["POST"],
    exposureClass: "public_derived",
    requiredRoles: [],
  },
  {
    kind: "template",
    targetPath: "map/v1/marker-masks/listing/:z/:x/:y",
    methods: ["GET"],
    exposureClass: "public_derived",
    requiredRoles: [],
  },
  {
    kind: "template",
    targetPath: "map/v1/marker-tombstones/listing/:z/:x/:y",
    methods: ["GET"],
    exposureClass: "public_derived",
    requiredRoles: [],
  },
  {
    kind: "template",
    targetPath: "map/v1/marker-deltas/listing/:z/:x/:y_pbf",
    methods: ["GET"],
    exposureClass: "public_derived",
    requiredRoles: [],
  },
  {
    kind: "exact",
    targetPath: "listings",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "exact",
    targetPath: "listings",
    methods: ["POST"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:id",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:id",
    methods: ["PATCH"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:id/submit-for-review",
    methods: ["POST"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:id/revise",
    methods: ["POST"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:id/bookmark",
    methods: ["POST", "DELETE"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-write",
      keyStrategy: "session_sub",
      limit: 120,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:id/photos",
    methods: ["POST"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:listing_id/photos/:photo_id",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:listing_id/photos/:photo_id",
    methods: ["DELETE"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "listings/:listing_id/photos/:photo_id/confirm",
    methods: ["POST"],
    exposureClass: "privileged",
    requiredRoles: ["Broker"],
    rate: {
      keyPrefix: "api-proxy:privileged-write",
      keyStrategy: "session_sub",
      limit: 60,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "exact",
    targetPath: "me/notifications/unread-count",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "me/notifications/:id/read",
    methods: ["PATCH"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-write",
      keyStrategy: "session_sub",
      limit: 120,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "exact",
    targetPath: "me/notifications/mark-all-read",
    methods: ["POST"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-write",
      keyStrategy: "session_sub",
      limit: 120,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "exact",
    targetPath: "me/bookmarks",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "template",
    targetPath: "api/parcels/:pnu",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
  {
    kind: "exact",
    targetPath: "api/buildings",
    methods: ["GET"],
    exposureClass: "authenticated_user",
    requiredRoles: [],
    rate: {
      keyPrefix: "api-proxy:authenticated-read",
      keyStrategy: "session_sub",
      limit: 240,
      windowSec: 60,
      problemType: "proxy/too-many-requests",
    },
  },
];

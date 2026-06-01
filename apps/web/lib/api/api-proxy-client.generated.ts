// Generated from docs/architecture/traffic-auth-policy-registry.v1.json.
// Run scripts/ci/generate-traffic-auth-policy.ps1 after editing the registry.

import type { Options as KyOptions } from "ky";
import { api } from "@/lib/api";

export type ApiProxyRequestOptions = Omit<KyOptions, "prefixUrl" | "method">;
export type ApiProxyJsonRequestOptions = Omit<
  KyOptions,
  "prefixUrl" | "method" | "body" | "json"
> & {
  readonly json?: unknown;
};

function encodePathParam(value: string): string {
  return encodeURIComponent(value);
}

function toJsonRequestOptions(options?: ApiProxyJsonRequestOptions): KyOptions | undefined {
  if (options === undefined) {
    return undefined;
  }
  const { json, ...rest } = options;
  if (json === undefined) {
    return rest;
  }
  return { ...rest, json };
}

export const API_PROXY_CLIENT_OPERATIONS = {
  publicMarkerTiles: {
    sourcePolicyId: "gongzzang.api_proxy.public_marker_tiles",
    targetPath: "map/v1/marker-tiles/listing/:z/:x/:y_pbf",
    methods: ["GET"],
  },
  publicMarkerCounts: {
    sourcePolicyId: "gongzzang.api_proxy.public_marker_counts",
    targetPath: "map/v1/marker-counts/listing",
    methods: ["GET"],
  },
  publicMarkerFilters: {
    sourcePolicyId: "gongzzang.api_proxy.public_marker_filters",
    targetPath: "map/v1/marker-filters/listing",
    methods: ["POST"],
  },
  publicMarkerMasks: {
    sourcePolicyId: "gongzzang.api_proxy.public_marker_masks",
    targetPath: "map/v1/marker-masks/listing/:z/:x/:y",
    methods: ["GET"],
  },
  publicMarkerTombstones: {
    sourcePolicyId: "gongzzang.api_proxy.public_marker_tombstones",
    targetPath: "map/v1/marker-tombstones/listing/:z/:x/:y",
    methods: ["GET"],
  },
  publicMarkerDeltas: {
    sourcePolicyId: "gongzzang.api_proxy.public_marker_deltas",
    targetPath: "map/v1/marker-deltas/listing/:z/:x/:y_pbf",
    methods: ["GET"],
  },
  listingsCollectionRead: {
    sourcePolicyId: "gongzzang.api_proxy.listings_collection_read",
    targetPath: "listings",
    methods: ["GET"],
  },
  listingsCollectionCreate: {
    sourcePolicyId: "gongzzang.api_proxy.listings_collection_create",
    targetPath: "listings",
    methods: ["POST"],
  },
  listingDetailRead: {
    sourcePolicyId: "gongzzang.api_proxy.listing_detail_read",
    targetPath: "listings/:id",
    methods: ["GET"],
  },
  listingDetailUpdate: {
    sourcePolicyId: "gongzzang.api_proxy.listing_detail_update",
    targetPath: "listings/:id",
    methods: ["PATCH"],
  },
  listingSubmitForReview: {
    sourcePolicyId: "gongzzang.api_proxy.listing_submit_for_review",
    targetPath: "listings/:id/submit-for-review",
    methods: ["POST"],
  },
  listingRevise: {
    sourcePolicyId: "gongzzang.api_proxy.listing_revise",
    targetPath: "listings/:id/revise",
    methods: ["POST"],
  },
  listingBookmark: {
    sourcePolicyId: "gongzzang.api_proxy.listing_bookmark",
    targetPath: "listings/:id/bookmark",
    methods: ["POST", "DELETE"],
  },
  listingPhotosCollection: {
    sourcePolicyId: "gongzzang.api_proxy.listing_photos_collection",
    targetPath: "listings/:id/photos",
    methods: ["POST"],
  },
  listingPhotoReadDelete: {
    sourcePolicyId: "gongzzang.api_proxy.listing_photo_read_delete",
    targetPath: "listings/:listing_id/photos/:photo_id",
    methods: ["GET"],
  },
  listingPhotoDelete: {
    sourcePolicyId: "gongzzang.api_proxy.listing_photo_delete",
    targetPath: "listings/:listing_id/photos/:photo_id",
    methods: ["DELETE"],
  },
  listingPhotoConfirm: {
    sourcePolicyId: "gongzzang.api_proxy.listing_photo_confirm",
    targetPath: "listings/:listing_id/photos/:photo_id/confirm",
    methods: ["POST"],
  },
  notificationsList: {
    sourcePolicyId: "gongzzang.api_proxy.notifications_list",
    targetPath: "me/notifications",
    methods: ["GET"],
  },
  notificationsUnreadCount: {
    sourcePolicyId: "gongzzang.api_proxy.notifications_unread_count",
    targetPath: "me/notifications/unread-count",
    methods: ["GET"],
  },
  notificationMarkRead: {
    sourcePolicyId: "gongzzang.api_proxy.notification_mark_read",
    targetPath: "me/notifications/:id/read",
    methods: ["PATCH"],
  },
  notificationsMarkAllRead: {
    sourcePolicyId: "gongzzang.api_proxy.notifications_mark_all_read",
    targetPath: "me/notifications/mark-all-read",
    methods: ["POST"],
  },
  myBookmarks: {
    sourcePolicyId: "gongzzang.api_proxy.my_bookmarks",
    targetPath: "me/bookmarks",
    methods: ["GET"],
  },
  parcelRead: {
    sourcePolicyId: "gongzzang.api_proxy.parcel_read",
    targetPath: "api/parcels/:pnu",
    methods: ["GET"],
  },
  buildingsRead: {
    sourcePolicyId: "gongzzang.api_proxy.buildings_read",
    targetPath: "api/buildings",
    methods: ["GET"],
  },
} as const;

export const apiProxyClient = {
  publicMarkerTiles: {
    get: (
      params: { readonly z: string; readonly x: string; readonly y_pbf: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api.get(
        `map/v1/marker-tiles/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y_pbf)}`,
        options,
      ),
    getJson: <T>(
      params: { readonly z: string; readonly x: string; readonly y_pbf: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api
        .get(
          `map/v1/marker-tiles/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y_pbf)}`,
          options,
        )
        .json<T>(),
  },
  publicMarkerCounts: {
    get: (options?: ApiProxyRequestOptions) => api.get("map/v1/marker-counts/listing", options),
    getJson: <T>(options?: ApiProxyRequestOptions) =>
      api.get("map/v1/marker-counts/listing", options).json<T>(),
  },
  publicMarkerFilters: {
    post: (options?: ApiProxyJsonRequestOptions) =>
      api.post("map/v1/marker-filters/listing", toJsonRequestOptions(options)),
    postJson: <T>(options?: ApiProxyJsonRequestOptions) =>
      api.post("map/v1/marker-filters/listing", toJsonRequestOptions(options)).json<T>(),
  },
  publicMarkerMasks: {
    get: (
      params: { readonly z: string; readonly x: string; readonly y: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api.get(
        `map/v1/marker-masks/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y)}`,
        options,
      ),
    getJson: <T>(
      params: { readonly z: string; readonly x: string; readonly y: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api
        .get(
          `map/v1/marker-masks/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y)}`,
          options,
        )
        .json<T>(),
  },
  publicMarkerTombstones: {
    get: (
      params: { readonly z: string; readonly x: string; readonly y: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api.get(
        `map/v1/marker-tombstones/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y)}`,
        options,
      ),
    getJson: <T>(
      params: { readonly z: string; readonly x: string; readonly y: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api
        .get(
          `map/v1/marker-tombstones/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y)}`,
          options,
        )
        .json<T>(),
  },
  publicMarkerDeltas: {
    get: (
      params: { readonly z: string; readonly x: string; readonly y_pbf: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api.get(
        `map/v1/marker-deltas/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y_pbf)}`,
        options,
      ),
    getJson: <T>(
      params: { readonly z: string; readonly x: string; readonly y_pbf: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api
        .get(
          `map/v1/marker-deltas/listing/${encodePathParam(params.z)}/${encodePathParam(params.x)}/${encodePathParam(params.y_pbf)}`,
          options,
        )
        .json<T>(),
  },
  listingsCollectionRead: {
    get: (options?: ApiProxyRequestOptions) => api.get("listings", options),
    getJson: <T>(options?: ApiProxyRequestOptions) => api.get("listings", options).json<T>(),
  },
  listingsCollectionCreate: {
    post: (options?: ApiProxyJsonRequestOptions) =>
      api.post("listings", toJsonRequestOptions(options)),
    postJson: <T>(options?: ApiProxyJsonRequestOptions) =>
      api.post("listings", toJsonRequestOptions(options)).json<T>(),
  },
  listingDetailRead: {
    get: (params: { readonly id: string }, options?: ApiProxyRequestOptions) =>
      api.get(`listings/${encodePathParam(params.id)}`, options),
    getJson: <T>(params: { readonly id: string }, options?: ApiProxyRequestOptions) =>
      api.get(`listings/${encodePathParam(params.id)}`, options).json<T>(),
  },
  listingDetailUpdate: {
    patch: (params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.patch(`listings/${encodePathParam(params.id)}`, toJsonRequestOptions(options)),
    patchJson: <T>(params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.patch(`listings/${encodePathParam(params.id)}`, toJsonRequestOptions(options)).json<T>(),
  },
  listingSubmitForReview: {
    post: (params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.post(
        `listings/${encodePathParam(params.id)}/submit-for-review`,
        toJsonRequestOptions(options),
      ),
    postJson: <T>(params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api
        .post(
          `listings/${encodePathParam(params.id)}/submit-for-review`,
          toJsonRequestOptions(options),
        )
        .json<T>(),
  },
  listingRevise: {
    post: (params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.post(`listings/${encodePathParam(params.id)}/revise`, toJsonRequestOptions(options)),
    postJson: <T>(params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api
        .post(`listings/${encodePathParam(params.id)}/revise`, toJsonRequestOptions(options))
        .json<T>(),
  },
  listingBookmark: {
    post: (params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.post(`listings/${encodePathParam(params.id)}/bookmark`, toJsonRequestOptions(options)),
    postJson: <T>(params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api
        .post(`listings/${encodePathParam(params.id)}/bookmark`, toJsonRequestOptions(options))
        .json<T>(),
    delete: (params: { readonly id: string }, options?: ApiProxyRequestOptions) =>
      api.delete(`listings/${encodePathParam(params.id)}/bookmark`, options),
    deleteJson: <T>(params: { readonly id: string }, options?: ApiProxyRequestOptions) =>
      api.delete(`listings/${encodePathParam(params.id)}/bookmark`, options).json<T>(),
  },
  listingPhotosCollection: {
    post: (params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.post(`listings/${encodePathParam(params.id)}/photos`, toJsonRequestOptions(options)),
    postJson: <T>(params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api
        .post(`listings/${encodePathParam(params.id)}/photos`, toJsonRequestOptions(options))
        .json<T>(),
  },
  listingPhotoReadDelete: {
    get: (
      params: { readonly listing_id: string; readonly photo_id: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api.get(
        `listings/${encodePathParam(params.listing_id)}/photos/${encodePathParam(params.photo_id)}`,
        options,
      ),
    getJson: <T>(
      params: { readonly listing_id: string; readonly photo_id: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api
        .get(
          `listings/${encodePathParam(params.listing_id)}/photos/${encodePathParam(params.photo_id)}`,
          options,
        )
        .json<T>(),
  },
  listingPhotoDelete: {
    delete: (
      params: { readonly listing_id: string; readonly photo_id: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api.delete(
        `listings/${encodePathParam(params.listing_id)}/photos/${encodePathParam(params.photo_id)}`,
        options,
      ),
    deleteJson: <T>(
      params: { readonly listing_id: string; readonly photo_id: string },
      options?: ApiProxyRequestOptions,
    ) =>
      api
        .delete(
          `listings/${encodePathParam(params.listing_id)}/photos/${encodePathParam(params.photo_id)}`,
          options,
        )
        .json<T>(),
  },
  listingPhotoConfirm: {
    post: (
      params: { readonly listing_id: string; readonly photo_id: string },
      options?: ApiProxyJsonRequestOptions,
    ) =>
      api.post(
        `listings/${encodePathParam(params.listing_id)}/photos/${encodePathParam(params.photo_id)}/confirm`,
        toJsonRequestOptions(options),
      ),
    postJson: <T>(
      params: { readonly listing_id: string; readonly photo_id: string },
      options?: ApiProxyJsonRequestOptions,
    ) =>
      api
        .post(
          `listings/${encodePathParam(params.listing_id)}/photos/${encodePathParam(params.photo_id)}/confirm`,
          toJsonRequestOptions(options),
        )
        .json<T>(),
  },
  notificationsList: {
    get: (options?: ApiProxyRequestOptions) => api.get("me/notifications", options),
    getJson: <T>(options?: ApiProxyRequestOptions) =>
      api.get("me/notifications", options).json<T>(),
  },
  notificationsUnreadCount: {
    get: (options?: ApiProxyRequestOptions) => api.get("me/notifications/unread-count", options),
    getJson: <T>(options?: ApiProxyRequestOptions) =>
      api.get("me/notifications/unread-count", options).json<T>(),
  },
  notificationMarkRead: {
    patch: (params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api.patch(
        `me/notifications/${encodePathParam(params.id)}/read`,
        toJsonRequestOptions(options),
      ),
    patchJson: <T>(params: { readonly id: string }, options?: ApiProxyJsonRequestOptions) =>
      api
        .patch(`me/notifications/${encodePathParam(params.id)}/read`, toJsonRequestOptions(options))
        .json<T>(),
  },
  notificationsMarkAllRead: {
    post: (options?: ApiProxyJsonRequestOptions) =>
      api.post("me/notifications/mark-all-read", toJsonRequestOptions(options)),
    postJson: <T>(options?: ApiProxyJsonRequestOptions) =>
      api.post("me/notifications/mark-all-read", toJsonRequestOptions(options)).json<T>(),
  },
  myBookmarks: {
    get: (options?: ApiProxyRequestOptions) => api.get("me/bookmarks", options),
    getJson: <T>(options?: ApiProxyRequestOptions) => api.get("me/bookmarks", options).json<T>(),
  },
  parcelRead: {
    get: (params: { readonly pnu: string }, options?: ApiProxyRequestOptions) =>
      api.get(`api/parcels/${encodePathParam(params.pnu)}`, options),
    getJson: <T>(params: { readonly pnu: string }, options?: ApiProxyRequestOptions) =>
      api.get(`api/parcels/${encodePathParam(params.pnu)}`, options).json<T>(),
  },
  buildingsRead: {
    get: (options?: ApiProxyRequestOptions) => api.get("api/buildings", options),
    getJson: <T>(options?: ApiProxyRequestOptions) => api.get("api/buildings", options).json<T>(),
  },
} as const;

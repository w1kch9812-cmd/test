// apps/web/lib/routes.ts
//
// Internal app routes / API paths SSOT. 모든 router push / form action / fetch
// 가 본 const 를 참조하여 *문자열 hardcode 0*.
//
// 사용처:
// - <Link href={ROUTES.notifications}>
// - <form action={API.auth.login}>
// - ky.create({ prefixUrl: API.proxy.base })
// - router.push(ROUTES.listings.detail(id))
//
// 외부 URL (Naver Maps SDK / V-World source link 등) 은 본 파일 *외부* 에 별도
// 정의 (도메인 의존).

const API_PROXY_BASE = "/api/proxy";

/** Internal page routes (사용자 navigate target). */
export const ROUTES = {
  home: "/",
  login: "/login",
  forbidden: "/forbidden",
  /** `/profile` (top-level route — `(authenticated)/profile/page.tsx`). M3 cutover
   * 후 `/me/profile` 로 이동 가능하나 현재는 legacy 위치. */
  profile: "/profile",
  listings: {
    index: "/listings",
    new: "/listings/new",
    detail: (id: string) => `/listings/${id}` as const,
    edit: (id: string) => `/listings/${id}/edit` as const,
  },
  me: {
    notifications: "/me/notifications",
  },
} as const;

/** Internal API routes (fetch target). */
export const API = {
  auth: {
    login: "/api/auth/login",
    logout: "/api/auth/logout",
    callback: "/api/auth/callback",
    refresh: "/api/auth/refresh",
  },
  proxy: {
    base: API_PROXY_BASE,
    listingMarkerCounts: `${API_PROXY_BASE}/map/v1/marker-counts/listing`,
    listingMarkerFilters: `${API_PROXY_BASE}/map/v1/marker-filters/listing`,
    listingMarkerDeltasPrefix: `${API_PROXY_BASE}/map/v1/marker-deltas/listing`,
    listingMarkerDeltaTemplate: `${API_PROXY_BASE}/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version={baseVersion}`,
    listingMarkerMasksPrefix: `${API_PROXY_BASE}/map/v1/marker-masks/listing`,
    listingMarkerMaskTemplate: `${API_PROXY_BASE}/map/v1/marker-masks/listing/{z}/{x}/{y}?filter_hash={hash}&base_version={baseVersion}`,
    listingMarkerTombstonesPrefix: `${API_PROXY_BASE}/map/v1/marker-tombstones/listing`,
    listingMarkerTombstoneTemplate: `${API_PROXY_BASE}/map/v1/marker-tombstones/listing/{z}/{x}/{y}?base_version={baseVersion}`,
    listingMarkerTilesPrefix: `${API_PROXY_BASE}/map/v1/marker-tiles/listing`,
    listingMarkerTileTemplate: `${API_PROXY_BASE}/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash={hash}`,
    listingPhoto: (listingId: string, photoId: string) =>
      `${API_PROXY_BASE}/listings/${encodeURIComponent(listingId)}/photos/${encodeURIComponent(
        photoId,
      )}` as const,
  },
  platformCore: {
    events: "/platform-core/events",
  },
} as const;

/** Auth prefix — proxy.ts 의 internal-only path 분류용. */
export const AUTH_PATH_PREFIX = "/api/auth";

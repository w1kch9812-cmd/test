export const GONGZZANG_MAP_ZOOM_POLICY = {
  platformCore: {
    exactParcelAnchorMinZoom: 12,
  },
  levels: {
    sido: {
      min: 0,
      max: 9,
    },
    sig: {
      min: 10,
      max: 11,
    },
    emd: {
      min: 12,
      max: 13,
    },
    parcel: {
      min: 14,
      max: 22,
    },
  },
  markers: {
    listing: {
      minZoom: 14,
      maxZoom: 22,
    },
  },
} as const;

export const LISTING_MARKER_RENDER_MIN_ZOOM = GONGZZANG_MAP_ZOOM_POLICY.markers.listing.minZoom;
export const LISTING_MARKER_RENDER_MAX_ZOOM = GONGZZANG_MAP_ZOOM_POLICY.markers.listing.maxZoom;

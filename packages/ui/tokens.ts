// packages/ui/tokens.ts
//
// JS 측 design token SSOT. CSS variable 정의 (packages/ui/tokens/*.css) 와
// *반드시 동기화*. 변경 시 양쪽 모두 update.

/**
 * Viewport breakpoints (px).
 *
 * Tailwind v4 default scale. CSS variable `--breakpoint-*` 와 동기화
 * (packages/ui/tokens/breakpoints.css).
 */
export const BREAKPOINTS_PX = {
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
  "2xl": 1536,
} as const;

export type Breakpoint = keyof typeof BREAKPOINTS_PX;

/** `(min-width: NNNpx)` 형식. useMediaQuery 등에 직접 전달. */
export const MEDIA_QUERIES = {
  sm: `(min-width: ${BREAKPOINTS_PX.sm}px)`,
  md: `(min-width: ${BREAKPOINTS_PX.md}px)`,
  lg: `(min-width: ${BREAKPOINTS_PX.lg}px)`,
  xl: `(min-width: ${BREAKPOINTS_PX.xl}px)`,
  "2xl": `(min-width: ${BREAKPOINTS_PX["2xl"]}px)`,
} as const satisfies Record<Breakpoint, string>;

/** `(max-width: (NNN-1)px)` 형식. Next.js Image sizes 등의 inverse query. */
export const MEDIA_QUERIES_MAX = {
  sm: `(max-width: ${BREAKPOINTS_PX.sm - 1}px)`,
  md: `(max-width: ${BREAKPOINTS_PX.md - 1}px)`,
  lg: `(max-width: ${BREAKPOINTS_PX.lg - 1}px)`,
  xl: `(max-width: ${BREAKPOINTS_PX.xl - 1}px)`,
  "2xl": `(max-width: ${BREAKPOINTS_PX["2xl"] - 1}px)`,
} as const satisfies Record<Breakpoint, string>;

// ────────────────────────────────────────────────────────────────────────────
// Listing domain colors
// CSS variable 동기화: packages/ui/tokens/listings.css
// 사용처: SVG fill/stroke, Mapbox/MapLibre paint property (CSS var 못 받음).
// 도메인 어휘: docs/glossary.md (factory=공장 등)
// ────────────────────────────────────────────────────────────────────────────

/** 매물 타입별 핀 색상 (Tailwind 600 hue scale). */
export const LISTING_TYPE_COLORS = {
  factory: "#dc2626", // red-600
  warehouse: "#2563eb", // blue-600
  office: "#059669", // emerald-600
  knowledge_industry_center: "#7c3aed", // violet-600
  industrial_land: "#ea580c", // orange-600
  logistics_center: "#0891b2", // cyan-600
} as const;

export type ListingTypeKey = keyof typeof LISTING_TYPE_COLORS;

/** 미지/누락 listing_type 의 fallback (gray-500). */
export const LISTING_TYPE_COLOR_FALLBACK = "#6b7280";

/** 지도 layer fill/outline 색상. parcels(core) / admin / complex 3 layer. */
export const MAP_LAYER_COLORS = {
  parcel: { fill: "#10b981", outline: "#059669" }, // emerald-500/600
  admin: { fill: "#9ca3af", outline: "#6b7280" }, // gray-400/500
  complex: { fill: "#3b82f6", outline: "#1d4ed8" }, // blue-500/700
} as const;

/** 핀 SVG stroke + 내부 색. */
export const PIN_COLORS = {
  strokeSelected: "#ffffff",
  strokeIdle: "#1f2937", // gray-800
  inner: "#ffffff",
} as const;

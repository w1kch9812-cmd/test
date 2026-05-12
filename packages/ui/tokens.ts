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

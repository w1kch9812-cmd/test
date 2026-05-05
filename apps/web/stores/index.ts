/**
 * Zustand stores 진입점.
 *
 * SP6-foundation: skeleton 만 (interface 분리 → 미래 Jotai 등 swap 가능).
 * SP6-i ~ v 가 실제 store 추가 (auth / search / bookmarks 등).
 */

export interface StoreInterface {
  // 미래 stores 가 implement (예: reset() / hydrate(state) 등)
  reset?: () => void;
}

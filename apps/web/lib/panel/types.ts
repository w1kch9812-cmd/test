// apps/web/lib/panel/types.ts

/**
 * SP10 Panel System — typed stack 추상화.
 * Spec § 3 Pattern E. 새 kind 추가 = `PanelKind` union 확장 + `PanelView<K>` 분기.
 * Framework 본체 (`lib/panel/*`) 는 kind 폴더 (`components/panels/*`) 를 import 하지 않음.
 */

export type PanelKind = "parcel" | "listing";

export type PanelView<K extends PanelKind> =
  | (K extends "parcel" ? "summary" | "buildings" | "listings" : never)
  | (K extends "listing" ? "summary" : never);

export type PanelStackEntry = {
  [K in PanelKind]: { kind: K; id: string; view: PanelView<K> };
}[PanelKind];

export interface PanelStack {
  v: 1;
  entries: PanelStackEntry[];
}

export const EMPTY_STACK: PanelStack = { v: 1, entries: [] };

/** depth 8 hard limit (spec § 14). */
export const PANEL_DEPTH_MAX = 8;
export const PANEL_DEPTH_WARN = 6;

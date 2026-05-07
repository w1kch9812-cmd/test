// apps/web/lib/panel/registry.ts
import type { ComponentType } from "react";
import type { PanelKind, PanelStackEntry, PanelView } from "./types";

/**
 * Spec § 6 — registry shape R1. SSOT for kind/view definitions.
 * Module-singleton: T4/T5 의 register.ts 가 import 시점에 1회 등록.
 */

export interface PanelLink<TFromData> {
  /** UI label key (i18n) */
  labelKey: string;
  /** discriminator: when this link should render (optional predicate on fetched data) */
  show?: (data: TFromData) => boolean;
  /** target stack entry — must reference *another* kind's registered view (compile-time enforced) */
  to: (data: TFromData) => PanelStackEntry;
}

export interface PanelViewDefinition<K extends PanelKind, TData = unknown> {
  component: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }>; data: TData }>;
  /** id → server fetch. Returns parsed data (zod schema 검증된 후). */
  fetcher: (id: string) => Promise<TData>;
  /** TanStack Query staleTime ms. spec FU2 = per-kind tune. v1 default 5min. */
  staleTime: number;
  /** child link list — registry link integrity 는 T4/T5 가 검증 */
  links: PanelLink<TData>[];
}

export interface PanelKindDefinition<K extends PanelKind> {
  kind: K;
  idPattern: RegExp;
  views: { [V in PanelView<K>]: PanelViewDefinition<K> };
  loadingComponent: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }> }>;
  errorComponent: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }>; error: unknown }>;
  emptyComponent: ComponentType<{ entry: Extract<PanelStackEntry, { kind: K }> }>;
  /** auth: required=true → 미인증 시 AuthGate 카드. */
  authGate: { required: boolean };
  i18nNamespace: string;
  /** Sentry breadcrumb / OTEL span attributes. */
  telemetryAttrs: (
    entry: Extract<PanelStackEntry, { kind: K }>,
  ) => Record<string, string | number | boolean>;
}

const REGISTRY = new Map<PanelKind, PanelKindDefinition<PanelKind>>();

export function defineKind<K extends PanelKind>(def: PanelKindDefinition<K>): void {
  if (REGISTRY.has(def.kind)) {
    throw new Error(`Panel kind '${def.kind}' is already registered`);
  }
  REGISTRY.set(def.kind, def as unknown as PanelKindDefinition<PanelKind>);
}

export function getKindDefinition<K extends PanelKind>(
  kind: K,
): PanelKindDefinition<K> | undefined {
  return REGISTRY.get(kind) as PanelKindDefinition<K> | undefined;
}

export function getView<K extends PanelKind>(
  kind: K,
  view: PanelView<K>,
): PanelViewDefinition<K> | undefined {
  const def = getKindDefinition(kind);
  return def?.views[view] as PanelViewDefinition<K> | undefined;
}

/** TEST ONLY — clears registry between tests. */
export function _resetRegistryForTests(): void {
  REGISTRY.clear();
}

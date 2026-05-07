// apps/web/lib/panel/codec.ts
import type { PanelKind, PanelStack, PanelStackEntry, PanelView } from "./types";
import { PANEL_DEPTH_MAX } from "./types";

/**
 * Spec § 5 — URL = SSOT. 모든 string 파싱은 본 파일만.
 * `string.split('>')` ad-hoc 파싱은 lefthook lint 가 차단 (T6).
 */

export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

export const ParseError = {
  Malformed: "malformed",
  UnknownKind: "unknown_kind",
  UnknownView: "unknown_view",
  IdPatternViolation: "id_pattern_violation",
  DepthExceeded: "depth_exceeded",
} as const;
export type ParseError = (typeof ParseError)[keyof typeof ParseError];

interface KindMeta {
  views: ReadonlySet<string>;
  idPattern: RegExp;
}

/** SSOT for kind regex + valid views. spec § 5.3 + § 6. */
const KINDS: Record<PanelKind, KindMeta> = {
  parcel: {
    views: new Set(["summary", "buildings", "listings"]),
    idPattern: /^\d{19}$/,
  },
  listing: {
    views: new Set(["summary"]),
    idPattern: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/,
  },
};

const VALID_KINDS = Object.keys(KINDS) as PanelKind[];

function isPanelKind(s: string): s is PanelKind {
  return (VALID_KINDS as string[]).includes(s);
}

export interface PanelStackCodec {
  CURRENT_VERSION: 1;
  serialize(stack: PanelStack): string;
  deserialize(s: string): Result<PanelStack, ParseError>;
}

function serializeEntry(e: PanelStackEntry): string {
  return `${e.kind}:${e.id}.${e.view}`;
}

function deserializeEntry(raw: string): Result<PanelStackEntry, ParseError> {
  // grammar: kind ':' id '.' view
  const colon = raw.indexOf(":");
  if (colon < 1) return { ok: false, error: ParseError.Malformed };
  const kind = raw.slice(0, colon);
  const rest = raw.slice(colon + 1);
  const lastDot = rest.lastIndexOf(".");
  if (lastDot < 1) return { ok: false, error: ParseError.Malformed };
  const id = rest.slice(0, lastDot);
  const view = rest.slice(lastDot + 1);
  if (!id || !view) return { ok: false, error: ParseError.Malformed };
  if (!isPanelKind(kind)) return { ok: false, error: ParseError.UnknownKind };
  const meta = KINDS[kind];
  if (!meta.views.has(view)) return { ok: false, error: ParseError.UnknownView };
  if (!meta.idPattern.test(id)) return { ok: false, error: ParseError.IdPatternViolation };
  // Type-safe assembly: discriminated union narrows view per kind.
  return { ok: true, value: { kind, id, view: view as PanelView<PanelKind> } as PanelStackEntry };
}

export const g1Codec: PanelStackCodec = {
  CURRENT_VERSION: 1,
  serialize(stack: PanelStack): string {
    return stack.entries.map(serializeEntry).join(">");
  },
  deserialize(s: string): Result<PanelStack, ParseError> {
    if (s === "") return { ok: true, value: { v: 1, entries: [] } };
    const parts = s.split(">");
    if (parts.length > PANEL_DEPTH_MAX) {
      return { ok: false, error: ParseError.DepthExceeded };
    }
    const entries: PanelStackEntry[] = [];
    for (const p of parts) {
      const r = deserializeEntry(p);
      if (!r.ok) return r;
      entries.push(r.value);
    }
    return { ok: true, value: { v: 1, entries } };
  },
};

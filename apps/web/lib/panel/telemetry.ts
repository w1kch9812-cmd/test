// apps/web/lib/panel/telemetry.ts
import { trace } from "@opentelemetry/api";
import type { ParseError } from "./codec";
import type { PanelStackEntry } from "./types";

/**
 * Spec § 10.4 — telemetry standard.
 * v1 backends:
 *   - OTEL span (panel.opened) — spec § 10.4 attributes
 *   - console.warn for url_decode_failed (Sentry adoption = drop-in replace)
 *   - window.dataLayer push (analytics)
 *
 * 본 helper 는 Sentry 도입 후 import 만 swap (call sites 동일).
 */

const TRACER = trace.getTracer("panel");

interface AnalyticsDataLayer {
  push: (event: Record<string, unknown>) => void;
}

interface DataLayerWindow extends Window {
  dataLayer?: AnalyticsDataLayer | Array<Record<string, unknown>>;
}

function pushAnalytics(event: Record<string, unknown>): void {
  if (typeof window === "undefined") return;
  const dl = (window as DataLayerWindow).dataLayer;
  if (Array.isArray(dl)) {
    dl.push(event);
  } else if (dl && typeof dl.push === "function") {
    dl.push(event);
  }
}

export function reportPanelOpened(entry: PanelStackEntry, depth: number, fetchMs: number): void {
  const span = TRACER.startSpan("panel.opened", {
    attributes: {
      "panel.kind": entry.kind,
      "panel.view": entry.view,
      "panel.id": entry.id,
      "panel.depth": depth,
      "panel.fetch_ms": fetchMs,
    },
  });
  span.end();

  pushAnalytics({
    event: "panel_opened",
    panel_kind: entry.kind,
    panel_view: entry.view,
    panel_id: entry.id,
    panel_depth: depth,
  });
}

export function reportUrlDecodeFailed(raw: string, error: ParseError): void {
  const span = TRACER.startSpan("panel.url_decode_failed", {
    attributes: { "panel.raw": raw, "panel.error": error },
  });
  span.end();
  // dev visibility — production 은 OTEL collector 가 export.
  if (process.env.NODE_ENV !== "production") {
    console.warn("[panel] url_decode_failed", { raw, error });
  }
}

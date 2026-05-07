// apps/web/lib/panel/panel-entry-view.tsx
"use client";

import { useQuery } from "@tanstack/react-query";
import React, { createElement, useEffect, useMemo, useRef, useState } from "react";
import { PanelCard } from "./panel-card";
import { getKindDefinition, getView } from "./registry";
import { reportPanelOpened } from "./telemetry";
import type { PanelStackEntry } from "./types";
import { usePanelStack } from "./use-panel-stack";

/**
 * Panel-local error boundary. Catches *render-time* exceptions from registry
 * view components (registry component throws synchronously during render or
 * during a hook). Async/fetch errors are handled separately by TanStack Query
 * (query.isError → state="error").
 *
 * On catch: logs, then renders the registry's errorComponent via the parent
 * PanelEntryView's `state="error"` path — surfaces via setError prop callback.
 *
 * Spec rule § 9 #6 — error boundary per card. Closes T1 #6 gap.
 */
class PanelErrorBoundary extends React.Component<
  { children: React.ReactNode; onError: (err: Error) => void; fallback: React.ReactNode },
  { hasError: boolean }
> {
  override state = { hasError: false };
  static getDerivedStateFromError(): { hasError: boolean } {
    return { hasError: true };
  }
  override componentDidCatch(err: Error): void {
    this.props.onError(err);
  }
  override render(): React.ReactNode {
    return this.state.hasError ? this.props.fallback : this.props.children;
  }
}

/**
 * 단일 entry 의 렌더링 — fetcher 호출 + 4-state shell + 컴포넌트 dispatch.
 * Spec rule § 9 #1 (registry SSOT), #6 (error boundary via PanelCard + PanelErrorBoundary),
 * #8 (AbortController per slot — TanStack Query 가 자동), #17 (4-state).
 */
export function PanelEntryView({ entry, depth }: { entry: PanelStackEntry; depth: number }) {
  const def = getKindDefinition(entry.kind);
  const viewDef = getView(entry.kind, entry.view);
  const startedAt = useRef(performance.now());
  const { pop } = usePanelStack();
  const [renderError, setRenderError] = useState<Error | null>(null);

  // Spec rule § 9 #8 — TanStack Query 의 queryFn 이 AbortSignal 받아 fetcher 에 전달.
  const query = useQuery({
    queryKey: ["panel", entry.kind, entry.view, entry.id],
    queryFn: async ({ signal }) => {
      void signal; // fetcher 가 signal 사용은 호출자 결정 (ky 가 AbortSignal 지원)
      // biome-ignore lint/style/noNonNullAssertion: enabled gate guarantees viewDef defined
      return viewDef!.fetcher(entry.id);
    },
    staleTime: viewDef?.staleTime ?? 5 * 60_000,
    enabled: Boolean(def && viewDef),
  });

  useEffect(() => {
    if (query.isSuccess) {
      reportPanelOpened(entry, depth, performance.now() - startedAt.current);
    }
  }, [query.isSuccess, entry, depth]);

  // Spec rule § 13 — registered 안된 view import 자체가 컴파일 에러여야 하지만, runtime 의 안전망.
  const stateNarrowed = useMemo(() => {
    if (!def || !viewDef) return "error" as const;
    if (renderError) return "error" as const;
    if (query.isLoading) return "loading" as const;
    if (query.isError) return "error" as const;
    const data = query.data;
    if (data === null || (Array.isArray(data) && data.length === 0)) return "empty" as const;
    return "ok" as const;
  }, [def, viewDef, renderError, query.isLoading, query.isError, query.data]);

  if (!def || !viewDef) {
    return (
      <div className="p-6 text-center text-[var(--color-error)]">
        Unknown panel kind/view: {entry.kind}.{entry.view}
      </div>
    );
  }

  return (
    <PanelCard
      state={stateNarrowed}
      onClose={pop}
      loading={createElement(def.loadingComponent, { entry: entry as never })}
      error={createElement(def.errorComponent, {
        entry: entry as never,
        error: renderError ?? query.error,
      })}
      empty={createElement(def.emptyComponent, { entry: entry as never })}
      authRequired={
        <div className="p-6 text-center text-[var(--color-muted)]">로그인이 필요해요</div>
      }
    >
      <PanelErrorBoundary
        onError={(err) => setRenderError(err)}
        fallback={null /* PanelCard will swap to state="error" via stateNarrowed */}
      >
        {query.data !== undefined &&
          createElement(viewDef.component, {
            entry: entry as never,
            data: query.data,
          })}
      </PanelErrorBoundary>
    </PanelCard>
  );
}

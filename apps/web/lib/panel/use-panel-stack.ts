// apps/web/lib/panel/use-panel-stack.ts
"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useMemo } from "react";
import { g1Codec } from "./codec";
import { reportUrlDecodeFailed } from "./telemetry";
import type { PanelStack, PanelStackEntry } from "./types";
import { EMPTY_STACK } from "./types";

/**
 * Spec § 5.4 — URL = SSOT. zustand 의 panelStack 사본은 *없음* — useSearchParams 직접.
 * mutation 은 router.push (URL grammar), pop 은 router.back (브라우저 stack).
 */

export interface UsePanelStackResult {
  stack: PanelStack;
  push: (entry: PanelStackEntry) => void;
  pop: () => void;
  /** stack 을 명시적 길이로 자름 (breadcrumb 클릭 시 사용). */
  truncate: (depth: number) => void;
}

export function usePanelStack(): UsePanelStackResult {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const raw = searchParams.get("p");

  const stack = useMemo<PanelStack>(() => {
    if (!raw) return EMPTY_STACK;
    const r = g1Codec.deserialize(raw);
    if (!r.ok) {
      reportUrlDecodeFailed(raw, r.error);
      return EMPTY_STACK;
    }
    return r.value;
  }, [raw]);

  const navigate = useCallback(
    (next: PanelStack) => {
      const sp = new URLSearchParams(searchParams.toString());
      const serialized = g1Codec.serialize(next);
      if (serialized) sp.set("p", serialized);
      else sp.delete("p");
      const qs = sp.toString();
      const url = `${pathname}${qs ? `?${qs}` : ""}`;
      // Next.js typed routes — the `?p=...` URL is dynamic, not statically routable.
      router.push(url as never, { scroll: false });
    },
    [pathname, router, searchParams],
  );

  const push = useCallback(
    (entry: PanelStackEntry) => {
      const next: PanelStack = { v: 1, entries: [...stack.entries, entry] };
      navigate(next);
    },
    [navigate, stack],
  );

  const pop = useCallback(() => {
    router.back();
  }, [router]);

  const truncate = useCallback(
    (depth: number) => {
      const safeDepth = Math.max(0, Math.min(depth, stack.entries.length));
      navigate({ v: 1, entries: stack.entries.slice(0, safeDepth) });
    },
    [navigate, stack],
  );

  return { stack, push, pop, truncate };
}

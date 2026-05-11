// apps/web/lib/panel/use-panel-stack.ts
"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback, useMemo } from "react";
import { g1Codec } from "./codec";
import { reportUrlDecodeFailed } from "./telemetry";
import type { PanelStack, PanelStackEntry } from "./types";
import { EMPTY_STACK, PANEL_DEPTH_MAX } from "./types";

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
    (next: PanelStack, mode: "push" | "replace") => {
      const sp = new URLSearchParams(searchParams.toString());
      const serialized = g1Codec.serialize(next);
      if (serialized) sp.set("p", serialized);
      else sp.delete("p");
      const qs = sp.toString();
      const url = `${pathname}${qs ? `?${qs}` : ""}`;
      // Next.js typed routes — the `?p=...` URL is dynamic, not statically routable.
      if (mode === "push") {
        router.push(url as never, { scroll: false });
      } else {
        router.replace(url as never, { scroll: false });
      }
    },
    [pathname, router, searchParams],
  );

  const push = useCallback(
    (entry: PanelStackEntry) => {
      if (stack.entries.length >= PANEL_DEPTH_MAX) {
        // 8 = hard limit. Spec § 14 — depth max 8 hard limit (warn at 6).
        // 9th push silently wipes via deserialize round-trip; refuse instead.
        if (process.env.NODE_ENV !== "production") {
          console.warn("[panel] push refused: depth limit reached", { max: PANEL_DEPTH_MAX });
        }
        return;
      }
      const next: PanelStack = { v: 1, entries: [...stack.entries, entry] };
      navigate(next, "push");
    },
    [navigate, stack],
  );

  /**
   * Pop the top panel entry.
   *
   * Fix #1 (2026-05-11): 이전 코드는 `router.back()` 만 호출 → 패널이 *직접 URL
   * 진입* (외부 링크 / 새 탭 / 북마크) 으로 열렸으면 brower history 에 in-app
   * entry 가 없어 *페이지 밖으로* navigate (이전 사이트 또는 about:blank). 사용자
   * 짜증의 주범.
   *
   * Fix: 현 stack 길이를 검사. depth > 0 이면 `replace` 로 stack 한 단계 잘라
   * navigation. router.back() 은 only when history depth > 1 (in-app push 가 있을 때).
   */
  const pop = useCallback(() => {
    if (stack.entries.length === 0) return;
    if (stack.entries.length === 1) {
      // 마지막 panel — `?p=` 자체 제거. URL 직접 진입 시 안전.
      const sp = new URLSearchParams(searchParams.toString());
      sp.delete("p");
      const qs = sp.toString();
      const url = `${pathname}${qs ? `?${qs}` : ""}`;
      router.replace(url as never, { scroll: false });
      return;
    }
    // depth > 1 — 한 단계 자르기. browser back 으로 안전하게 복귀 가능.
    const next: PanelStack = {
      v: 1,
      entries: stack.entries.slice(0, -1),
    };
    navigate(next, "replace");
  }, [navigate, pathname, router, searchParams, stack]);

  /**
   * Stack 을 명시적 길이로 자름 (breadcrumb 클릭 시 사용).
   *
   * 시맨틱: `router.replace` 사용 — forward history 를 늘리지 않음.
   * 진짜 pop 은 표준 web 으로 불가 (history.go(-N) 은 cross-origin 등 fragile).
   * Replace 가 차선책: 사용자가 system back 누르면 truncate 이전 상태로 복귀.
   */
  const truncate = useCallback(
    (depth: number) => {
      const safeDepth = Math.max(0, Math.min(depth, stack.entries.length));
      navigate({ v: 1, entries: stack.entries.slice(0, safeDepth) }, "replace");
    },
    [navigate, stack],
  );

  return { stack, push, pop, truncate };
}

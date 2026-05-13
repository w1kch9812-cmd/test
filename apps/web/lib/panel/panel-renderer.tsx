// apps/web/lib/panel/panel-renderer.tsx
"use client";

import { MEDIA_QUERIES } from "@gongzzang/ui/tokens.js";
import { useEffect, useState } from "react";
import "@/components/panels/listing/register";
import "@/components/panels/parcel/register";
import { FullScreenStack } from "./full-screen-stack";
import { SideBySideStack } from "./side-by-side-stack";
import { usePanelStack } from "./use-panel-stack";

/**
 * Spec § 4 — xl breakpoint 단일 분기. *그 외 어떤 컴포넌트에도 viewport 분기 코드 없음.*
 * 1280px 값은 @gongzzang/ui/tokens.js 의 BREAKPOINTS_PX SSOT.
 */
const XL_QUERY = MEDIA_QUERIES.xl;

function useIsDesktop(): boolean {
  const [isDesktop, setIsDesktop] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia(XL_QUERY);
    setIsDesktop(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsDesktop(e.matches);
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);
  return isDesktop;
}

export function PanelRenderer() {
  const isDesktop = useIsDesktop();
  const { stack } = usePanelStack();
  if (stack.entries.length === 0) return null;
  return isDesktop ? <SideBySideStack stack={stack} /> : <FullScreenStack stack={stack} />;
}

// apps/web/lib/panel/focus-trap.ts
"use client";

import { type RefObject, useEffect } from "react";

/**
 * Spec rule § 9.14 — focus push on open / restore on close.
 * Container must be focusable (tabIndex=-1 acceptable).
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const previously = document.activeElement as HTMLElement | null;
    ref.current?.focus();
    return () => {
      previously?.focus();
    };
  }, [ref]);
}

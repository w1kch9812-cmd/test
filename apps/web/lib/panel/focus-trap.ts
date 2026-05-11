// apps/web/lib/panel/focus-trap.ts
"use client";

import { type RefObject, useEffect } from "react";

/**
 * Spec rule § 9.14 — focus push on open / restore on close + Tab cycle trap.
 *
 * Fix #4 (2026-05-11) — 이전 구현은 *focus push/restore 만* 처리, Tab 키로
 * 컨테이너 밖으로 빠져나가는 것 차단 안 됨 (ARIA APG dialog 패턴 위반). 본 hook
 * 은 Tab/Shift+Tab 인터셉트 → 첫 ↔ 마지막 focusable element 사이 cycle 강제.
 */
export function useFocusTrap(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const container = ref.current;
    if (!container) return;

    const previously = document.activeElement as HTMLElement | null;
    container.focus();

    function getFocusable(): HTMLElement[] {
      if (!container) return [];
      // ARIA APG 권장 selector — 표준 focusable elements + tabindex=0.
      const selector =
        'a[href], area[href], input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), button:not([disabled]), iframe, [tabindex]:not([tabindex="-1"])';
      return Array.from(container.querySelectorAll<HTMLElement>(selector)).filter((el) => {
        // 숨김/비활성 element 제외
        if (el.getAttribute("aria-hidden") === "true") return false;
        const style = window.getComputedStyle(el);
        return style.display !== "none" && style.visibility !== "hidden";
      });
    }

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key !== "Tab") return;
      const focusable = getFocusable();
      if (focusable.length === 0) {
        // 어떤 focusable 도 없으면 container 자체 — Tab 자동 cycle 못 함, 막음.
        e.preventDefault();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (!first || !last) return;
      const active = document.activeElement as HTMLElement | null;

      if (e.shiftKey) {
        // Shift+Tab — 첫 element 에서 누르면 마지막으로 wrap
        if (active === first || active === container) {
          e.preventDefault();
          last.focus();
        }
      } else {
        // Tab — 마지막 element 에서 누르면 첫 번째로 wrap
        if (active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }

    container.addEventListener("keydown", handleKeyDown);
    return () => {
      container.removeEventListener("keydown", handleKeyDown);
      previously?.focus();
    };
  }, [ref]);
}

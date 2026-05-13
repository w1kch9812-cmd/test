"use client";

import { type RefObject, useEffect } from "react";

const FOCUSABLE_SELECTOR =
  'a[href], area[href], input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), button:not([disabled]), iframe, [tabindex]:not([tabindex="-1"])';

function isVisibleFocusableElement(element: HTMLElement): boolean {
  if (element.getAttribute("aria-hidden") === "true") return false;
  const style = window.getComputedStyle(element);
  return style.display !== "none" && style.visibility !== "hidden";
}

function getFocusableElements(container: HTMLElement): HTMLElement[] {
  return Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    isVisibleFocusableElement,
  );
}

function getFocusWrapTarget(
  event: KeyboardEvent,
  container: HTMLElement,
  focusable: readonly HTMLElement[],
): HTMLElement | null {
  const first = focusable[0];
  const last = focusable.at(-1);
  if (!first || !last) return container;

  const active = document.activeElement;
  if (event.shiftKey) {
    return active === first || active === container ? last : null;
  }
  return active === last ? first : null;
}

export function useFocusTrap(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const container = ref.current;
    if (!container) return;
    const trapRoot = container;

    const previously = document.activeElement as HTMLElement | null;
    trapRoot.focus();

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key !== "Tab") return;
      const target = getFocusWrapTarget(event, trapRoot, getFocusableElements(trapRoot));
      if (!target) return;
      event.preventDefault();
      target.focus();
    }

    trapRoot.addEventListener("keydown", handleKeyDown);
    return () => {
      trapRoot.removeEventListener("keydown", handleKeyDown);
      previously?.focus();
    };
  }, [ref]);
}

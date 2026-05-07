// apps/web/lib/panel/panel-card.tsx
"use client";

import { type ReactNode, useEffect, useRef } from "react";
import { useFocusTrap } from "./focus-trap";

/**
 * Spec rule § 9 #6 (error boundary), #14 (focus trap), #15 (ESC), #16 (reduced motion), #17 (4-state).
 * 4-state: loading / error / ok / empty / auth-required.
 *   (auth-required 가 별도 prop — registry 의 authGate 미통과 시 렌더)
 */

export type PanelCardState = "loading" | "error" | "empty" | "ok" | "auth-required";

export interface PanelCardProps {
  state: PanelCardState;
  onClose: () => void;
  loading: ReactNode;
  error: ReactNode;
  empty: ReactNode;
  authRequired: ReactNode;
  children: ReactNode;
  /** aria-labelledby target id (for screen readers). */
  titleId?: string;
}

export function PanelCard({
  state,
  onClose,
  loading,
  error,
  empty,
  authRequired,
  children,
  titleId,
}: PanelCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  useFocusTrap(ref);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  const body =
    state === "loading"
      ? loading
      : state === "error"
        ? error
        : state === "empty"
          ? empty
          : state === "auth-required"
            ? authRequired
            : children;

  return (
    <div
      ref={ref}
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      tabIndex={-1}
      // motion-safe / motion-reduce: spec § 9 #16
      className="motion-safe:animate-in motion-safe:slide-in-from-right motion-reduce:animate-none flex h-full w-full flex-col bg-[var(--color-canvas)]"
    >
      {body}
    </div>
  );
}

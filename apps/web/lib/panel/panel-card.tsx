// apps/web/lib/panel/panel-card.tsx
"use client";

import { type ReactNode, useEffect, useRef } from "react";
import { useFocusTrap } from "./focus-trap";

/**
 * Spec rule § 9 #14 (focus trap), #15 (ESC), #16 (reduced motion), #17 (4-state).
 * 4-state: loading / error / ok / empty / auth-required.
 *   (auth-required 가 별도 prop — registry 의 authGate 미통과 시 렌더)
 *
 * Spec rule § 9 #6 (error boundary per card) is implemented in
 * panel-entry-view.tsx — `PanelErrorBoundary` wraps the registry view component
 * within this PanelCard's children slot, flipping `state="error"` on catch.
 */

export type PanelCardState = "loading" | "error" | "empty" | "ok" | "auth-required";

interface PanelCardBaseProps {
  state: PanelCardState;
  onClose: () => void;
  closeOnEscape?: boolean;
  loading: ReactNode;
  error: ReactNode;
  empty: ReactNode;
  authRequired: ReactNode;
  children: ReactNode;
}

type PanelCardAccessibleName =
  | {
      /** aria-labelledby target id (for screen readers). */
      titleId: string;
      ariaLabel?: never;
    }
  | {
      titleId?: undefined;
      /** Accessible dialog name when the rendered title is not a stable element. */
      ariaLabel: string;
    };

export type PanelCardProps = PanelCardBaseProps & PanelCardAccessibleName;

export function PanelCard({
  state,
  onClose,
  closeOnEscape = true,
  loading,
  error,
  empty,
  authRequired,
  children,
  titleId,
  ariaLabel,
}: PanelCardProps) {
  const ref = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  useFocusTrap(ref);

  useEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  useEffect(() => {
    if (!closeOnEscape) return;

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onCloseRef.current();
    }
    document.addEventListener("keydown", onKey, { capture: true });
    return () => document.removeEventListener("keydown", onKey, { capture: true });
  }, [closeOnEscape]);

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
      aria-label={titleId ? undefined : ariaLabel}
      tabIndex={-1}
      // motion-safe / motion-reduce: spec § 9 #16
      className="motion-safe:animate-in motion-safe:slide-in-from-right motion-reduce:animate-none flex h-full w-full flex-col bg-[var(--color-canvas)]"
    >
      {body}
    </div>
  );
}

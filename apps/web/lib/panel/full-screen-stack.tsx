// apps/web/lib/panel/full-screen-stack.tsx
"use client";

import { ChevronLeft } from "lucide-react";
import { useTranslations } from "next-intl";
import { PanelEntryView } from "./panel-entry-view";
import type { PanelStack } from "./types";
import { usePanelStack } from "./use-panel-stack";

/**
 * Spec § 4 mobile renderer. top 1 entry full-screen + 상단 ‹back + depth indicator.
 * back 은 router.back (브라우저 hw back / iOS edge-swipe 와 동등).
 */
export function FullScreenStack({ stack }: { stack: PanelStack }) {
  const total = stack.entries.length;
  const t = useTranslations("panel");
  const { pop } = usePanelStack();
  if (total === 0) return null;

  // biome-ignore lint/style/noNonNullAssertion: total > 0 above guarantees index exists
  const top = stack.entries[total - 1]!;

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-[var(--color-canvas)]">
      <div className="flex items-center gap-2 border-b border-[var(--color-hairline)] px-4 py-3">
        <button
          type="button"
          onClick={pop}
          aria-label={t("back")}
          className="flex h-9 w-9 items-center justify-center rounded-full hover:bg-[var(--color-surface-cream-strong)]"
        >
          <ChevronLeft className="h-5 w-5" />
        </button>
        <span className="text-[length:var(--text-caption)] text-[var(--color-muted)]">
          {total} / {total}
        </span>
      </div>
      <div className="flex-1 overflow-y-auto">
        <PanelEntryView entry={top} depth={total} />
      </div>
    </div>
  );
}

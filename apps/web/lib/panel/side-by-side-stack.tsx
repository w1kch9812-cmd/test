// apps/web/lib/panel/side-by-side-stack.tsx
"use client";

import { Breadcrumb } from "./breadcrumb";
import { PanelEntryView } from "./panel-entry-view";
import type { PanelStack } from "./types";

/**
 * Spec § 4 desktop renderer. depth ≥ xl 에서 top 2 entry 를 side-by-side.
 * depth 3+ = sliding window (마지막 2 만), breadcrumb 회색 항목으로 이전 표시.
 *
 * 위치: `fixed top-0 right-0 bottom-0` overlay — 페이지 grid 와
 * 독립적이라 listings page 의 map / card list aside 는 영향 0.
 * Width SSOT: --panel-side-by-side-width (packages/ui/tokens/listings.css).
 */
export function SideBySideStack({ stack }: { stack: PanelStack }) {
  const total = stack.entries.length;
  if (total === 0) return null;

  const top2Start = Math.max(0, total - 2);
  const visible = stack.entries.slice(top2Start);

  return (
    <div
      className="fixed top-0 right-0 bottom-0 z-40 flex flex-col border-l border-[var(--color-hairline)] bg-[var(--color-canvas)] shadow-xl"
      style={{ width: "var(--panel-side-by-side-width)" }}
    >
      <Breadcrumb stack={stack} greyedBeforeIndex={top2Start} />
      <div className="grid flex-1 grid-cols-2 gap-4 overflow-hidden">
        {visible.map((entry, i) => (
          <PanelEntryView
            key={`${entry.kind}-${entry.id}-${entry.view}`}
            entry={entry}
            depth={top2Start + i + 1}
            isTop={i === visible.length - 1}
          />
        ))}
      </div>
    </div>
  );
}

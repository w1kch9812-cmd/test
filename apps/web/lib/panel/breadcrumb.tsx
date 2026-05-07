// apps/web/lib/panel/breadcrumb.tsx
"use client";

import { useTranslations } from "next-intl";
import { getKindDefinition } from "./registry";
import type { PanelStack } from "./types";
import { usePanelStack } from "./use-panel-stack";

interface BreadcrumbProps {
  stack: PanelStack;
  /** 회색 항목 (sliding window 밖) 시작 인덱스. desktop 만 사용. -1 = no greyed. */
  greyedBeforeIndex?: number;
}

export function Breadcrumb({ stack, greyedBeforeIndex = -1 }: BreadcrumbProps) {
  const t = useTranslations("panel");
  const { truncate } = usePanelStack();

  if (stack.entries.length === 0) return null;

  return (
    <nav
      aria-label={t("breadcrumb")}
      className="flex items-center gap-1 px-4 py-2 text-[length:var(--text-caption)]"
    >
      {stack.entries.map((entry, idx) => {
        const def = getKindDefinition(entry.kind);
        const isLast = idx === stack.entries.length - 1;
        const greyed = greyedBeforeIndex >= 0 && idx < greyedBeforeIndex;
        const label = def ? `${entry.kind}.${entry.view}` : entry.kind;
        return (
          <span key={`${entry.kind}-${entry.id}-${entry.view}`} className="flex items-center gap-1">
            {idx > 0 && <span className="text-[var(--color-muted)]">/</span>}
            <button
              type="button"
              onClick={() => truncate(idx + 1)}
              disabled={isLast}
              className={[
                "rounded px-1 hover:bg-[var(--color-surface-cream-strong)]",
                greyed ? "text-[var(--color-muted)]" : "text-[var(--color-ink)]",
                isLast ? "cursor-default font-semibold" : "cursor-pointer",
              ].join(" ")}
              aria-current={isLast ? "page" : undefined}
            >
              {label}
            </button>
          </span>
        );
      })}
    </nav>
  );
}

"use client";

import { useTranslations } from "next-intl";
import { getKindDefinition } from "./registry";
import type { PanelStack, PanelStackEntry } from "./types";
import { usePanelStack } from "./use-panel-stack";

interface BreadcrumbProps {
  stack: PanelStack;
  /** Desktop sliding-window entries before this index are dimmed. */
  greyedBeforeIndex?: number;
}

interface BreadcrumbItemProps {
  entry: PanelStackEntry;
  index: number;
  isLast: boolean;
  isDimmed: boolean;
  label: string;
  onSelect: (depth: number) => void;
}

function fallbackLabel(kind: PanelStackEntry["kind"]): string {
  return kind === "parcel" ? "필지" : "매물";
}

function breadcrumbLabel(entry: PanelStackEntry, translateLabel: (key: string) => string): string {
  if (!getKindDefinition(entry.kind)) return fallbackLabel(entry.kind);

  try {
    return translateLabel(`${entry.kind}.${entry.view}`);
  } catch {
    return fallbackLabel(entry.kind);
  }
}

function BreadcrumbItem({ entry, index, isLast, isDimmed, label, onSelect }: BreadcrumbItemProps) {
  return (
    <span key={`${entry.kind}-${entry.id}-${entry.view}`} className="flex items-center gap-1">
      {index > 0 ? <span className="text-[var(--color-muted)]">/</span> : null}
      <button
        type="button"
        onClick={() => onSelect(index + 1)}
        disabled={isLast}
        className={[
          "rounded px-1 hover:bg-[var(--color-surface-cream-strong)]",
          isDimmed ? "text-[var(--color-muted)]" : "text-[var(--color-ink)]",
          isLast ? "cursor-default font-semibold" : "cursor-pointer",
        ].join(" ")}
        aria-current={isLast ? "page" : undefined}
      >
        {label}
      </button>
    </span>
  );
}

export function Breadcrumb({ stack, greyedBeforeIndex = -1 }: BreadcrumbProps) {
  const t = useTranslations("panel");
  const tLabels = useTranslations("panel.labels");
  const { truncate } = usePanelStack();

  if (stack.entries.length === 0) return null;

  return (
    <nav
      aria-label={t("breadcrumb")}
      className="flex items-center gap-1 px-4 py-2 text-[length:var(--text-caption)]"
    >
      {stack.entries.map((entry, index) => (
        <BreadcrumbItem
          key={`${entry.kind}-${entry.id}-${entry.view}`}
          entry={entry}
          index={index}
          isLast={index === stack.entries.length - 1}
          isDimmed={greyedBeforeIndex >= 0 && index < greyedBeforeIndex}
          label={breadcrumbLabel(entry, tLabels)}
          onSelect={truncate}
        />
      ))}
    </nav>
  );
}

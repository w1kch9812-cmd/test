"use client";
import { cn } from "../lib/utils";

/*
 * MultiSelect — Claude.com spec 의 category-tab / category-tab-active 패턴 응용.
 * 칩 토글 UI: inactive=transparent + muted text, active=cream-card surface + ink text.
 * pill 모양 (radius-pill) 으로 가벼운 분위기, padding 6px×14px.
 */
interface MultiSelectOption {
  value: string;
  label: string;
}

interface MultiSelectProps {
  options: MultiSelectOption[];
  value: string[];
  onValueChange: (next: string[]) => void;
  className?: string;
}

export function MultiSelect({ options, value, onValueChange, className }: MultiSelectProps) {
  const toggle = (v: string) => {
    if (value.includes(v)) onValueChange(value.filter((x) => x !== v));
    else onValueChange([...value, v]);
  };
  return (
    <div className={cn("flex flex-wrap gap-1.5", className)}>
      {options.map((opt) => {
        const selected = value.includes(opt.value);
        return (
          <button
            key={opt.value}
            type="button"
            aria-pressed={selected}
            onClick={() => toggle(opt.value)}
            className={cn(
              "rounded-[var(--radius-pill)] px-3.5 py-1.5 text-[length:var(--text-caption)] font-medium transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]/30 focus-visible:ring-offset-2",
              selected
                ? "bg-[var(--color-ink)] text-[var(--color-on-dark)]"
                : "border border-[var(--color-hairline)] bg-[var(--color-canvas)] text-[var(--color-body)] hover:bg-[var(--color-surface-soft)]",
            )}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

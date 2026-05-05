"use client";
import * as SliderPrimitive from "@radix-ui/react-slider";
import { cn } from "../lib/utils";

/*
 * RangeSlider — Radix Slider 기반.
 * Claude spec 에 슬라이더 컴포넌트는 명시되지 않아 색만 시스템에 맞춤:
 * track=hairline, range=ink (강조는 색이 아니라 명도로), thumb=canvas + hairline.
 */
interface RangeSliderProps {
  min: number;
  max: number;
  step?: number;
  value: [number, number];
  onValueChange: (next: [number, number]) => void;
  formatValue?: (v: number) => string;
  className?: string;
}

export function RangeSlider({
  min,
  max,
  step = 1,
  value,
  onValueChange,
  formatValue,
  className,
}: RangeSliderProps) {
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <SliderPrimitive.Root
        min={min}
        max={max}
        step={step}
        value={value}
        onValueChange={(v) => onValueChange([v[0], v[1]] as [number, number])}
        className="relative flex h-5 w-full touch-none select-none items-center"
      >
        <SliderPrimitive.Track className="relative h-1 w-full grow overflow-hidden rounded-full bg-[var(--color-hairline)]">
          <SliderPrimitive.Range className="absolute h-full bg-[var(--color-ink)]" />
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb
          className="block h-4 w-4 rounded-full border border-[var(--color-hairline)] bg-[var(--color-canvas)] shadow-[var(--shadow-soft)] focus:outline-none focus:ring-2 focus:ring-[var(--color-primary)]/30"
          aria-label="최소값"
        />
        <SliderPrimitive.Thumb
          className="block h-4 w-4 rounded-full border border-[var(--color-hairline)] bg-[var(--color-canvas)] shadow-[var(--shadow-soft)] focus:outline-none focus:ring-2 focus:ring-[var(--color-primary)]/30"
          aria-label="최대값"
        />
      </SliderPrimitive.Root>
      <div className="flex justify-between text-[length:var(--text-caption-uppercase)] text-[var(--color-muted)]">
        <span>{formatValue ? formatValue(value[0]) : value[0]}</span>
        <span>{formatValue ? formatValue(value[1]) : value[1]}</span>
      </div>
    </div>
  );
}

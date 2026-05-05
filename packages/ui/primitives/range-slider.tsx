"use client";
import * as SliderPrimitive from "@radix-ui/react-slider";
import { cn } from "../lib/utils";

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
    <div className={cn("flex flex-col gap-2", className)}>
      <SliderPrimitive.Root
        min={min}
        max={max}
        step={step}
        value={value}
        onValueChange={(v) => onValueChange([v[0], v[1]] as [number, number])}
        className="relative flex h-5 w-full touch-none select-none items-center"
      >
        <SliderPrimitive.Track className="relative h-1 w-full grow overflow-hidden rounded-full bg-[var(--color-muted)]">
          <SliderPrimitive.Range className="absolute h-full bg-[var(--color-brand-600)]" />
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-[var(--color-brand-600)] bg-[var(--color-bg)] shadow focus:outline-none focus:ring-2 focus:ring-[var(--color-brand-500)]" />
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-[var(--color-brand-600)] bg-[var(--color-bg)] shadow focus:outline-none focus:ring-2 focus:ring-[var(--color-brand-500)]" />
      </SliderPrimitive.Root>
      <div className="flex justify-between text-xs text-[var(--color-muted-fg)]">
        <span>{formatValue ? formatValue(value[0]) : value[0]}</span>
        <span>{formatValue ? formatValue(value[1]) : value[1]}</span>
      </div>
    </div>
  );
}

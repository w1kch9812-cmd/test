import * as React from "react";
import { cn } from "../lib/utils";

/*
 * Skeleton — 로딩 자리 placeholder. surface-soft 배경 + 약한 pulse.
 */
export const Skeleton = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "animate-pulse rounded-[var(--radius-md)] bg-[var(--color-surface-soft)]",
        className,
      )}
      {...props}
    />
  ),
);
Skeleton.displayName = "Skeleton";

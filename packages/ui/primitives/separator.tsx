import * as React from "react";
import { cn } from "../lib/utils";

/*
 * Separator — hairline 색의 1px 시각적 구분선.
 * role="none" 으로 a11y 트리에서 제외 (장식용). 의미있는 구분이 필요한 경우
 * (예: settings 패널의 movable splitter) 에는 Radix Separator 도입 검토.
 */
interface SeparatorProps extends React.HTMLAttributes<HTMLDivElement> {
  orientation?: "horizontal" | "vertical";
}

export const Separator = React.forwardRef<HTMLDivElement, SeparatorProps>(
  ({ className, orientation = "horizontal", ...props }, ref) => (
    <div
      ref={ref}
      role="none"
      className={cn(
        "bg-[var(--color-hairline)]",
        orientation === "horizontal" ? "h-px w-full" : "h-full w-px",
        className,
      )}
      {...props}
    />
  ),
);
Separator.displayName = "Separator";

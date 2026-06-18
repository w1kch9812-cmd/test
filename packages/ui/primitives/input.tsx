import * as React from "react";
import { cn } from "../lib/utils";

/*
 * Input primitive for dense listing and operations forms.
 * 40px height, hairline border, body-md text. focus 시 coral ring (3px / 15% alpha).
 */
export type InputProps = React.InputHTMLAttributes<HTMLInputElement>;

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => (
    <input
      type={type}
      className={cn(
        "flex h-10 w-full rounded-[var(--radius-md)] border border-[var(--color-hairline)] bg-[var(--color-canvas)] px-3.5 py-2 text-[length:var(--text-body-md)] text-[var(--color-ink)]",
        "placeholder:text-[var(--color-muted)]",
        "focus:border-[var(--color-primary)] focus:outline-none focus:ring-[3px] focus:ring-[var(--color-primary)]/15",
        "disabled:cursor-not-allowed disabled:opacity-50",
        "file:border-0 file:bg-transparent file:text-sm file:font-medium",
        className,
      )}
      ref={ref}
      {...props}
    />
  ),
);
Input.displayName = "Input";

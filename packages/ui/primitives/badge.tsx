import { cva, type VariantProps } from "class-variance-authority";
import type * as React from "react";
import { cn } from "../lib/utils";

/*
 * Badge variants for compact status and category labels.
 * - default: surface-card 배경, ink text, caption (13px / 500), pill 모양
 * - coral: primary 배경, on-primary text, uppercase caption (12px / 500 / tracking 0)
 * - outline: hairline border, body text
 */
const badgeVariants = cva(
  "inline-flex items-center rounded-[var(--radius-pill)] px-3 py-1 transition-colors",
  {
    variants: {
      variant: {
        default:
          "bg-[var(--color-surface-card)] text-[var(--color-ink)] text-[length:var(--text-caption)] font-medium",
        coral:
          "bg-[var(--color-primary)] text-[var(--color-on-primary)] text-[length:var(--text-caption-uppercase)] font-medium uppercase tracking-[var(--tracking-uppercase)]",
        outline:
          "border border-[var(--color-hairline)] bg-[var(--color-canvas)] text-[var(--color-body)] text-[length:var(--text-caption)] font-medium",
        success:
          "bg-[var(--color-success)]/15 text-[var(--color-success)] text-[length:var(--text-caption)] font-medium",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export const Badge = ({ className, variant, ...props }: BadgeProps) => (
  <span className={cn(badgeVariants({ variant, className }))} {...props} />
);

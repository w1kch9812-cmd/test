import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "../lib/utils";

/*
 * Button variants for Gongzzang work surfaces.
 *
 * 기본: rounded-md (8px), height 40px, type-button (14px / 500 / tracking 0).
 * coral 은 primary 에만. press 시 primary-active 로 어두워짐.
 * Secondary and ghost variants keep the interface quiet for repeated operational use.
 */
const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-primary)]/30 focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--color-canvas)] disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        primary:
          "bg-[var(--color-primary)] text-[var(--color-on-primary)] active:bg-[var(--color-primary-active)]",
        secondary:
          "border border-[var(--color-hairline)] bg-[var(--color-canvas)] text-[var(--color-ink)] hover:bg-[var(--color-surface-soft)]",
        "secondary-on-dark":
          "bg-[var(--color-surface-dark-elevated)] text-[var(--color-on-dark)] hover:bg-[var(--color-surface-dark-soft)]",
        ghost: "text-[var(--color-ink)] hover:bg-[var(--color-surface-soft)]",
        link: "text-[var(--color-primary)] underline-offset-4 hover:underline",
        destructive: "bg-[var(--color-error)] text-white active:opacity-90",
      },
      size: {
        default: "h-10 rounded-[var(--radius-md)] px-5 text-[length:var(--text-button)]",
        sm: "h-9 rounded-[var(--radius-sm)] px-3 text-[length:var(--text-button)]",
        lg: "h-11 rounded-[var(--radius-md)] px-8 text-[length:var(--text-button)]",
        icon: "h-9 w-9 rounded-[var(--radius-full)] border border-[var(--color-hairline)] bg-[var(--color-canvas)]",
      },
    },
    defaultVariants: { variant: "primary", size: "default" },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp className={cn(buttonVariants({ variant, size, className }))} ref={ref} {...props} />
    );
  },
);
Button.displayName = "Button";

export { buttonVariants };

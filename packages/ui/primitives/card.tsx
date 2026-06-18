import * as React from "react";
import { cn } from "../lib/utils";

/*
 * Card surfaces for repeated product information blocks.
 * Surface 색은 prop 으로 받지 않고 className 으로 override.
 *
 * - 기본: canvas surface, hairline border
 * - "cream-card" surface: soft cream background, no border
 * - "dark" surface: high-contrast work surface
 */
type CardSurface = "default" | "cream-card" | "dark";

const surfaceClass: Record<CardSurface, string> = {
  default: "bg-[var(--color-canvas)] text-[var(--color-ink)] border border-[var(--color-hairline)]",
  "cream-card": "bg-[var(--color-surface-card)] text-[var(--color-ink)]",
  dark: "bg-[var(--color-surface-dark)] text-[var(--color-on-dark)]",
};

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  surface?: CardSurface;
}

export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className, surface = "default", ...props }, ref) => (
    <div
      ref={ref}
      className={cn("rounded-[var(--radius-lg)]", surfaceClass[surface], className)}
      {...props}
    />
  ),
);
Card.displayName = "Card";

export const CardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex flex-col gap-1.5 p-8", className)} {...props} />
  ),
);
CardHeader.displayName = "CardHeader";

export const CardTitle = React.forwardRef<
  HTMLHeadingElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className, ...props }, ref) => (
  <h3
    ref={ref}
    className={cn(
      "text-[length:var(--text-title-md)] font-semibold leading-[var(--leading-title)]",
      className,
    )}
    {...props}
  />
));
CardTitle.displayName = "CardTitle";

export const CardDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className, ...props }, ref) => (
  <p
    ref={ref}
    className={cn("text-[length:var(--text-body-sm)] text-[var(--color-muted)]", className)}
    {...props}
  />
));
CardDescription.displayName = "CardDescription";

export const CardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("p-8 pt-0", className)} {...props} />
  ),
);
CardContent.displayName = "CardContent";

export const CardFooter = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex items-center p-8 pt-0", className)} {...props} />
  ),
);
CardFooter.displayName = "CardFooter";

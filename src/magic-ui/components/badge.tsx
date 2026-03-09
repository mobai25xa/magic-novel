import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const badgeVariants = cva("badge", {
  variants: {
    variant: {
      solid: "badge-solid",
      soft: "badge-soft",
      outline: "badge-outline",
    },
    color: {
      default: "badge-default",
      primary: "badge-primary",
      success: "badge-success",
      warning: "badge-warning",
      error: "badge-error",
      info: "badge-info",
    },
    size: {
      sm: "badge-sm",
      default: "badge-md",
    },
    dot: {
      true: "badge-dot",
      false: "",
    },
  },
  defaultVariants: {
    variant: "soft",
    color: "default",
    size: "default",
    dot: false,
  },
})

export interface BadgeProps
  extends Omit<React.HTMLAttributes<HTMLSpanElement>, "color">,
    VariantProps<typeof badgeVariants> {}

const Badge = React.forwardRef<HTMLSpanElement, BadgeProps>(
  ({ className, variant, color, size, dot, ...props }, ref) => {
    return (
      <span
        ref={ref}
        className={cn(badgeVariants({ variant, color, size, dot }), className)}
        {...props}
      />
    )
  }
)
Badge.displayName = "Badge"

export { Badge, badgeVariants }

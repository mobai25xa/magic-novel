import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const spinnerVariants = cva("spinner", {
  variants: {
    size: {
      xs: "spinner-xs",
      sm: "spinner-sm",
      default: "",
      lg: "spinner-lg",
    },
    color: {
      default: "",
      muted: "spinner-muted",
      success: "spinner-success",
      white: "spinner-white",
    },
  },
  defaultVariants: {
    size: "default",
    color: "default",
  },
})

export interface SpinnerProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "color">,
    VariantProps<typeof spinnerVariants> {
  overlay?: boolean
}

const Spinner = React.forwardRef<HTMLDivElement, SpinnerProps>(
  ({ className, size, color, overlay = false, ...props }, ref) => {
    if (overlay) {
      return (
        <div ref={ref} className={cn("spinner-overlay", className)} {...props}>
          <div className={cn(spinnerVariants({ size, color }))} />
        </div>
      )
    }

    return <div ref={ref} className={cn(spinnerVariants({ size, color }), className)} {...props} />
  }
)
Spinner.displayName = "Spinner"

export { Spinner, spinnerVariants }

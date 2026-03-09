import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const chipVariants = cva("chip", {
  variants: {
    variant: {
      default: "",
      pastel: "chip-pastel",
    },
  },
  defaultVariants: {
    variant: "default",
  },
})

export interface ChipProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof chipVariants> {
  onRemove?: () => void
  children: React.ReactNode
}

const Chip = React.forwardRef<HTMLSpanElement, ChipProps>(
  ({ className, variant, onRemove, children, ...props }, ref) => {
    return (
      <span ref={ref} className={cn(chipVariants({ variant }), className)} {...props}>
        {children}
        {onRemove ? (
          <span
            role="button"
            aria-label="Remove chip"
            tabIndex={0}
            onClick={(event) => {
              event.stopPropagation()
              onRemove()
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault()
                event.stopPropagation()
                onRemove()
              }
            }}
          >
            ×
          </span>
        ) : null}
      </span>
    )
  },
)
Chip.displayName = "Chip"

export { Chip, chipVariants }

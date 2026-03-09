import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const toggleVariants = cva("toggle-switch", {
  variants: {
    size: {
      sm: "toggle-switch-sm",
      default: "",
      lg: "toggle-switch-lg",
    },
  },
  defaultVariants: {
    size: "default",
  },
})

export interface ToggleProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type" | "size">,
    VariantProps<typeof toggleVariants> {
  label?: string
}

const Toggle = React.forwardRef<HTMLInputElement, ToggleProps>(
  ({ className, label, size, disabled, ...props }, ref) => {
    const switchElement = (
      <label className={cn(toggleVariants({ size }), className)}>
        <input ref={ref} type="checkbox" disabled={disabled} {...props} />
        <span className="toggle-slider" />
      </label>
    )

    if (!label) {
      return switchElement
    }

    return (
      <div className="toggle-wrapper">
        <span className="toggle-label">{label}</span>
        {switchElement}
      </div>
    )
  },
)
Toggle.displayName = "Toggle"

export { Toggle, toggleVariants }

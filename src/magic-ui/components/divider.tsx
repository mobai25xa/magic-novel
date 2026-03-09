import * as React from "react"

import { cn } from "@/lib/utils"

export interface DividerProps extends React.HTMLAttributes<HTMLDivElement> {
  label?: string
}

const Divider = React.forwardRef<HTMLDivElement, DividerProps>(
  ({ className, label, ...props }, ref) => {
    if (label) {
      return (
        <div ref={ref} className={cn("section-divider", className)} role="separator" {...props}>
          {label}
        </div>
      )
    }

    return <div ref={ref} className={cn("section-divider-line", className)} role="separator" {...props} />
  }
)
Divider.displayName = "Divider"

export { Divider }

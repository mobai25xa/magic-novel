import * as React from "react"

import { cn } from "@/lib/utils"

export type AiChartShellProps = React.HTMLAttributes<HTMLDivElement>

const AiChartShell = React.forwardRef<HTMLDivElement, AiChartShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("rounded-md border overflow-hidden my-2", className)}
        {...props}
      />
    )
  }
)
AiChartShell.displayName = "AiChartShell"

export { AiChartShell }

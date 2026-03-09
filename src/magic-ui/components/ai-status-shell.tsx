import * as React from "react"

import { cn } from "@/lib/utils"

export type AiStatusShellProps = React.HTMLAttributes<HTMLDivElement>

const AiStatusShell = React.forwardRef<HTMLDivElement, AiStatusShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("rounded-md border border-border bg-background-80", className)}
        {...props}
      />
    )
  }
)
AiStatusShell.displayName = "AiStatusShell"

export { AiStatusShell }

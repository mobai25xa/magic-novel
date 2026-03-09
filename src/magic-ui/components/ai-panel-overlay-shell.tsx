import * as React from "react"

import { cn } from "@/lib/utils"

export type AiPanelOverlayShellProps = React.HTMLAttributes<HTMLDivElement>

const AiPanelOverlayShell = React.forwardRef<HTMLDivElement, AiPanelOverlayShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("absolute inset-0 z-20 backdrop-blur-sm border-l bg-background-95", className)}
        {...props}
      />
    )
  }
)
AiPanelOverlayShell.displayName = "AiPanelOverlayShell"

export { AiPanelOverlayShell }

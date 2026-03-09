import * as React from "react"

import { cn } from "@/lib/utils"

export type AiPanelHeaderShellProps = React.HTMLAttributes<HTMLDivElement>

const AiPanelHeaderShell = React.forwardRef<HTMLDivElement, AiPanelHeaderShellProps>(
  ({ className, ...props }, ref) => {
    return <div ref={ref} className={cn("ai-message-header", className)} {...props} />
  }
)
AiPanelHeaderShell.displayName = "AiPanelHeaderShell"

export { AiPanelHeaderShell }

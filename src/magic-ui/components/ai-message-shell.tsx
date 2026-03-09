import * as React from "react"

import { cn } from "@/lib/utils"

export type AiMessageShellProps = React.HTMLAttributes<HTMLDivElement>

const AiMessageShell = React.forwardRef<HTMLDivElement, AiMessageShellProps>(
  ({ className, ...props }, ref) => {
    return <div ref={ref} className={cn("ai-message", className)} {...props} />
  }
)
AiMessageShell.displayName = "AiMessageShell"

export { AiMessageShell }

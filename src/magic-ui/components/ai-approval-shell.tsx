import * as React from "react"

import { cn } from "@/lib/utils"

export type AiApprovalShellProps = React.HTMLAttributes<HTMLElement>

const AiApprovalShell = React.forwardRef<HTMLElement, AiApprovalShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <section
        ref={ref}
        className={cn(
          "space-y-3 rounded-md border border-ai-approval-border bg-ai-approval-bg px-3 py-3",
          className,
        )}
        {...props}
      />
    )
  }
)
AiApprovalShell.displayName = "AiApprovalShell"

export { AiApprovalShell }

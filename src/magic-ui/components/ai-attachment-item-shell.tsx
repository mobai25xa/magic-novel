import * as React from "react"

import { cn } from "@/lib/utils"

export type AiAttachmentItemShellProps = React.HTMLAttributes<HTMLDivElement>

const AiAttachmentItemShell = React.forwardRef<HTMLDivElement, AiAttachmentItemShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("flex items-start gap-2 rounded-md border bg-secondary-30 px-2.5 py-2 text-xs", className)}
        {...props}
      />
    )
  }
)
AiAttachmentItemShell.displayName = "AiAttachmentItemShell"

export { AiAttachmentItemShell }

import * as React from "react"

import { cn } from "@/lib/utils"

export type AiFlyoutShellProps = React.HTMLAttributes<HTMLDivElement>

const AiFlyoutShell = React.forwardRef<HTMLDivElement, AiFlyoutShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("rounded-lg border bg-background shadow-lg", className)}
        {...props}
      />
    )
  }
)
AiFlyoutShell.displayName = "AiFlyoutShell"

export { AiFlyoutShell }

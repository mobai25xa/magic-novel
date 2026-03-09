import * as React from "react"

import { cn } from "@/lib/utils"

export type AiScrollJumpShellProps = React.ButtonHTMLAttributes<HTMLButtonElement>

const AiScrollJumpShell = React.forwardRef<HTMLButtonElement, AiScrollJumpShellProps>(
  ({ className, type = "button", ...props }, ref) => {
    return (
      <button
        ref={ref}
        type={type}
        className={cn(
          "rounded-full border bg-card px-3 py-1 text-[11px] shadow-sm hover-bg inline-flex items-center gap-1",
          className,
        )}
        {...props}
      />
    )
  }
)
AiScrollJumpShell.displayName = "AiScrollJumpShell"

export { AiScrollJumpShell }

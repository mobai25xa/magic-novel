import * as React from "react"

import { cn } from "@/lib/utils"

export type AiPanelCardShellProps = React.HTMLAttributes<HTMLDivElement>

const AiPanelCardShell = React.forwardRef<HTMLDivElement, AiPanelCardShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("flex flex-col gap-3 rounded-lg border text-sm", className)}
        {...props}
      />
    )
  }
)
AiPanelCardShell.displayName = "AiPanelCardShell"

export type AiPanelIconButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement>

const AiPanelIconButton = React.forwardRef<HTMLButtonElement, AiPanelIconButtonProps>(
  ({ className, type = "button", ...props }, ref) => {
    return (
      <button
        ref={ref}
        type={type}
        className={cn("p-1 rounded hover:bg-secondary text-muted-foreground text-xs", className)}
        {...props}
      />
    )
  }
)
AiPanelIconButton.displayName = "AiPanelIconButton"

export { AiPanelCardShell, AiPanelIconButton }

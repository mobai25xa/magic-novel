import * as React from "react"

import { cn } from "@/lib/utils"

export type AiToolCardShellProps = React.HTMLAttributes<HTMLDivElement>

const AiToolCardShell = React.forwardRef<HTMLDivElement, AiToolCardShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("ai-tool-card", className)}
        {...props}
      />
    )
  }
)
AiToolCardShell.displayName = "AiToolCardShell"

export type AiToolCardHeaderButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement>

const AiToolCardHeaderButton = React.forwardRef<HTMLButtonElement, AiToolCardHeaderButtonProps>(
  ({ className, type = "button", ...props }, ref) => {
    return (
      <button
        ref={ref}
        type={type}
        className={cn(
          "ai-tool-card-header w-full border-0 text-left",
          className,
        )}
        {...props}
      />
    )
  }
)
AiToolCardHeaderButton.displayName = "AiToolCardHeaderButton"

export { AiToolCardShell, AiToolCardHeaderButton }

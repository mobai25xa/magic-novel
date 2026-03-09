import * as React from "react"

import { cn } from "@/lib/utils"

export type AiErrorShellProps = React.HTMLAttributes<HTMLDivElement>

const AiErrorShell = React.forwardRef<HTMLDivElement, AiErrorShellProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("rounded-md border border-ai-error-30 overflow-hidden", className)}
        {...props}
      />
    )
  }
)
AiErrorShell.displayName = "AiErrorShell"

export type AiErrorHeaderButtonProps = React.ButtonHTMLAttributes<HTMLButtonElement>

const AiErrorHeaderButton = React.forwardRef<HTMLButtonElement, AiErrorHeaderButtonProps>(
  ({ className, type = "button", ...props }, ref) => {
    return (
      <button
        ref={ref}
        type={type}
        className={cn(
          "flex w-full items-center gap-2 cursor-pointer px-2 py-1.5 select-none bg-ai-tool-card-header-bg hover:bg-ai-tool-card-bg transition-colors border-0 text-left",
          className,
        )}
        {...props}
      />
    )
  }
)
AiErrorHeaderButton.displayName = "AiErrorHeaderButton"

export type AiErrorBodyProps = React.HTMLAttributes<HTMLDivElement>

const AiErrorBody = React.forwardRef<HTMLDivElement, AiErrorBodyProps>(
  ({ className, ...props }, ref) => {
    return <div ref={ref} className={cn("px-2 py-1.5 border-t", className)} {...props} />
  }
)
AiErrorBody.displayName = "AiErrorBody"

export { AiErrorShell, AiErrorHeaderButton, AiErrorBody }

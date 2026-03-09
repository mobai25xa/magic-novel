import * as React from "react"

import { cn } from "@/lib/utils"

export type AiToolContentProps = React.HTMLAttributes<HTMLDivElement>

const AiToolContent = React.forwardRef<HTMLDivElement, AiToolContentProps>(
  ({ className, ...props }, ref) => {
    return (
      <div
        ref={ref}
        className={cn("px-3 py-2 border-t border-ai-tool-card-border", className)}
        {...props}
      />
    )
  }
)
AiToolContent.displayName = "AiToolContent"

export { AiToolContent }

import * as React from "react"

import { cn } from "@/lib/utils"

import { CodeBlock, type CodeBlockProps } from "./code-block"

export type AiCodePreviewProps = CodeBlockProps

const AiCodePreview = React.forwardRef<HTMLPreElement, AiCodePreviewProps>(
  ({ className, ...props }, ref) => {
    return (
      <CodeBlock
        ref={ref}
        className={cn("rounded border bg-background p-2 text-[11px] whitespace-pre-wrap break-words", className)}
        {...props}
      />
    )
  }
)
AiCodePreview.displayName = "AiCodePreview"

export { AiCodePreview }

import * as React from "react"

import { cn } from "@/lib/utils"

export interface CodeBlockProps extends React.HTMLAttributes<HTMLPreElement> {
  children: React.ReactNode
}

const CodeBlock = React.forwardRef<HTMLPreElement, CodeBlockProps>(
  ({ className, children, ...props }, ref) => {
    return (
      <pre ref={ref} className={cn("code-block", className)} {...props}>
        {children}
      </pre>
    )
  }
)
CodeBlock.displayName = "CodeBlock"

export { CodeBlock }

import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { AlertCircle, AlertTriangle, CheckCircle, Info } from "lucide-react"

import { cn } from "@/lib/utils"

const calloutVariants = cva("callout", {
  variants: {
    variant: {
      info: "",
      success: "callout-success",
      warning: "callout-warning",
      destructive: "callout-destructive",
    },
  },
  defaultVariants: {
    variant: "info",
  },
})

const calloutIcons = {
  info: <Info className="h-4 w-4" />,
  success: <CheckCircle className="h-4 w-4" />,
  warning: <AlertTriangle className="h-4 w-4" />,
  destructive: <AlertCircle className="h-4 w-4" />,
} satisfies Record<NonNullable<CalloutProps["variant"]>, React.ReactNode>

export interface CalloutProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof calloutVariants> {
  icon?: React.ReactNode
  children: React.ReactNode
}

const Callout = React.forwardRef<HTMLDivElement, CalloutProps>(
  ({ className, variant, icon, children, ...props }, ref) => {
    const currentVariant = variant ?? "info"

    return (
      <div ref={ref} className={cn(calloutVariants({ variant: currentVariant }), className)} {...props}>
        <span className="callout-icon" aria-hidden="true">
          {icon ?? calloutIcons[currentVariant]}
        </span>
        <div className="callout-content">{children}</div>
      </div>
    )
  }
)
Callout.displayName = "Callout"

export { Callout, calloutVariants }

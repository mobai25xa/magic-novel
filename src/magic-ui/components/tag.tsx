import * as React from "react"
import { X } from "lucide-react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const tagVariants = cva("tag", {
  variants: {
    variant: {
      default: "tag-default",
      secondary: "tag-secondary",
      outline: "tag-outline",
      success: "tag-success",
      warning: "tag-warning",
      destructive: "tag-destructive",
      info: "tag-info",
      "outline-success": "tag-outline tag-success",
      "outline-warning": "tag-outline tag-warning",
      "outline-destructive": "tag-outline tag-destructive",
      "outline-info": "tag-outline tag-info",
    },
    size: {
      sm: "tag-sm",
      default: "",
      lg: "tag-lg",
    },
  },
  defaultVariants: {
    variant: "default",
    size: "default",
  },
})

export interface TagProps
  extends React.HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof tagVariants> {
  closable?: boolean
  onClose?: () => void
}

const Tag = React.forwardRef<HTMLSpanElement, TagProps>(
  ({ className, variant, size, closable, onClose, children, ...props }, ref) => {
    return (
      <span
        ref={ref}
        className={cn(tagVariants({ variant, size }), closable && "tag-closable", className)}
        {...props}
      >
        {children}
        {closable ? (
          <button
            type="button"
            className="tag-close-btn"
            aria-label="Close tag"
            onClick={(event) => {
              event.stopPropagation()
              onClose?.()
            }}
          >
            <X className="h-3 w-3" />
          </button>
        ) : null}
      </span>
    )
  }
)
Tag.displayName = "Tag"

export { Tag, tagVariants }

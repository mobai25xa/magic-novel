import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const skeletonVariants = cva("skeleton", {
  variants: {
    variant: {
      text: "skeleton-text",
      heading: "skeleton-heading",
      avatar: "skeleton-avatar",
      rect: "skeleton-rect",
      button: "skeleton-button",
    },
  },
})

export interface SkeletonProps
  extends Omit<React.HTMLAttributes<HTMLDivElement>, "children">,
    VariantProps<typeof skeletonVariants> {
  width?: string | number
  height?: string | number
  lines?: number
}

function toCssSize(value: string | number | undefined) {
  if (typeof value === "number") {
    return `${value}px`
  }

  return value
}

const Skeleton = React.forwardRef<HTMLDivElement, SkeletonProps>(
  ({ className, variant, width, height, lines = 1, style, ...props }, ref) => {
    const mergedStyle: React.CSSProperties = {
      ...style,
      ...(width != null ? { width: toCssSize(width) } : null),
      ...(height != null ? { height: toCssSize(height) } : null),
    }

    if (variant === "text" && lines > 1) {
      return (
        <div ref={ref} className={className} {...props}>
          {Array.from({ length: lines }).map((_, index) => (
            <div
              key={index}
              className={cn(skeletonVariants({ variant: "text" }))}
              style={mergedStyle}
            />
          ))}
        </div>
      )
    }

    return (
      <div
        ref={ref}
        className={cn(skeletonVariants({ variant }), className)}
        style={mergedStyle}
        {...props}
      />
    )
  }
)
Skeleton.displayName = "Skeleton"

export { Skeleton, skeletonVariants }

import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const containerVariants = cva("container-base", {
  variants: {
    size: {
      tight: "container-tight",
      default: "container-default",
      wide: "container-wide",
      full: "container-full",
    },
  },
  defaultVariants: {
    size: "default",
  },
})

export interface ContainerProps
  extends React.HTMLAttributes<HTMLElement>,
    VariantProps<typeof containerVariants> {
  as?: React.ElementType
}

const Container = React.forwardRef<HTMLElement, ContainerProps>(
  ({ className, size, as: Component = "div", ...props }, ref) => {
    return (
      <Component
        ref={ref as React.Ref<HTMLElement>}
        className={cn(containerVariants({ size }), className)}
        {...props}
      />
    )
  }
)
Container.displayName = "Container"

export { Container, containerVariants }

import * as React from "react"
import * as TooltipPrimitive from "@radix-ui/react-tooltip"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const TooltipProvider = TooltipPrimitive.Provider
const Tooltip = TooltipPrimitive.Root
const TooltipTrigger = TooltipPrimitive.Trigger

const tooltipVariants = cva("tooltip-content", {
  variants: {
    variant: {
      default: "tooltip-variant-default",
      primary: "tooltip-variant-primary",
      secondary: "tooltip-variant-secondary",
      success: "tooltip-variant-success",
      warning: "tooltip-variant-warning",
      destructive: "tooltip-variant-destructive",
      info: "tooltip-variant-info",
      light: "tooltip-variant-light",
    },
    size: {
      sm: "tooltip-sm",
      default: "tooltip-default",
      lg: "tooltip-lg",
    },
  },
  defaultVariants: {
    variant: "default",
    size: "default",
  },
})

interface TooltipContentProps
  extends React.ComponentPropsWithoutRef<typeof TooltipPrimitive.Content>,
    VariantProps<typeof tooltipVariants> {
  showArrow?: boolean
  arrowClassName?: string
}


const TooltipContent = React.forwardRef<
  React.ElementRef<typeof TooltipPrimitive.Content>,
  TooltipContentProps
>(
  (
    {
      className,
      sideOffset = 4,
      variant,
      size,
      showArrow = false,
      arrowClassName,
      children,
      ...props
    },
    ref
  ) => {
    const resolvedVariant = variant ?? "default"

    return (
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          ref={ref}
          sideOffset={showArrow ? sideOffset + 4 : sideOffset}
          className={cn(tooltipVariants({ variant: resolvedVariant, size }), className)}
          {...props}
        >
          {children}
          {showArrow && (
            <TooltipPrimitive.Arrow
              className={cn("tooltip-arrow", arrowClassName)}
              width={10}
              height={5}
            />
          )}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    )
  }
)
TooltipContent.displayName = "TooltipContent"

export {
  Tooltip,
  TooltipTrigger,
  TooltipContent,
  TooltipProvider,
  tooltipVariants,
}

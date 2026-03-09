import * as React from "react"
import { Slot } from "@radix-ui/react-slot"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const buttonVariants = cva(
  "btn mui-button",
  {
    variants: {
      variant: {
        default: "btn-default mui-button--default",
        destructive: "btn-destructive mui-button--destructive",
        outline: "btn-outline mui-button--outline",
        settingsOutline: "mui-button--settings-outline",
        secondary: "btn-secondary mui-button--secondary",
        ghost: "btn-ghost mui-button--ghost",
        link: "btn-link mui-button--link",
      },
      size: {
        default: "",
        sm: "btn-sm mui-button--sm",
        lg: "btn-lg mui-button--lg",
        icon: "btn-icon mui-button--icon",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
)

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button"
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    )
  }
)
Button.displayName = "Button"

export { Button, buttonVariants }

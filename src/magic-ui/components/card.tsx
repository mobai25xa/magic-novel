import * as React from "react"
import { X } from "lucide-react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const cardVariants = cva("card", {
  variants: {
    padding: {
      none: "card-padding-none",
      sm: "card-padding-sm",
      default: "",
      lg: "card-padding-lg",
    },
  },
  defaultVariants: {
    padding: "default",
  },
})

export type CardProps = React.HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof cardVariants>

export interface CardHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  title?: string
  description?: string
  closable?: boolean
  onClose?: () => void
}

type CardFooterProps = React.HTMLAttributes<HTMLDivElement>

const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className, padding, ...props }, ref) => {
    return <div ref={ref} className={cn(cardVariants({ padding }), className)} {...props} />
  }
)
Card.displayName = "Card"

const CardHeader = React.forwardRef<HTMLDivElement, CardHeaderProps>(
  ({ className, title, description, closable, onClose, children, ...props }, ref) => {
    return (
      <div ref={ref} className={cn("card-header", className)} {...props}>
        {title ? <h2>{title}</h2> : null}
        {description ? <p>{description}</p> : null}
        {children}
        {closable ? (
          <button
            type="button"
            className="close-btn"
            aria-label="Close card"
            onClick={(event) => {
              event.stopPropagation()
              onClose?.()
            }}
          >
            <X className="h-4 w-4" />
          </button>
        ) : null}
      </div>
    )
  }
)
CardHeader.displayName = "CardHeader"

const CardFooter = React.forwardRef<HTMLDivElement, CardFooterProps>(
  ({ className, ...props }, ref) => {
    return <div ref={ref} className={cn("card-footer", className)} {...props} />
  }
)
CardFooter.displayName = "CardFooter"

export { Card, CardHeader, CardFooter, cardVariants }

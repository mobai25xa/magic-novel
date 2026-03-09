import * as React from "react"
import * as SliderPrimitive from "@radix-ui/react-slider"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const sliderVariants = cva("slider-root", {
  variants: {
    size: {
      sm: "slider-sm",
      default: "slider-default",
      lg: "slider-lg",
    },
  },
  defaultVariants: {
    size: "default",
  },
})

export interface SliderProps
  extends React.ComponentPropsWithoutRef<typeof SliderPrimitive.Root>,
    VariantProps<typeof sliderVariants> {
  showValue?: boolean
  formatValue?: (value: number) => string
}

const Slider = React.forwardRef<React.ElementRef<typeof SliderPrimitive.Root>, SliderProps>(
  ({ className, size, showValue, formatValue = (v) => String(v), value, defaultValue, ...props }, ref) => {
    const currentValue = value || defaultValue || [0]

    return (
      <div className="slider-wrapper">
        <SliderPrimitive.Root
          ref={ref}
          className={cn(sliderVariants({ size }), className)}
          value={value}
          defaultValue={defaultValue}
          {...props}
        >
          <SliderPrimitive.Track className="slider-track">
            <SliderPrimitive.Range className="slider-range" />
          </SliderPrimitive.Track>
          {currentValue.map((_, index) => (
            <SliderPrimitive.Thumb key={index} className="slider-thumb" />
          ))}
        </SliderPrimitive.Root>
        {showValue ? (
          <div className="slider-value-row">
            {currentValue.map((v, index) => (
              <span key={index}>{formatValue(v)}</span>
            ))}
          </div>
        ) : null}
      </div>
    )
  },
)

Slider.displayName = "Slider"

export { Slider, sliderVariants }

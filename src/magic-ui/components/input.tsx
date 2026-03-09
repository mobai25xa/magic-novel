import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { Search, Eye, EyeOff, X } from "lucide-react"

import { cn } from "@/lib/utils"

import { Button } from "./button"

const inputVariants = cva(
  "flex w-full transition-all duration-200 file:border-0 file:bg-transparent file:text-sm file:font-medium focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "",
        outline: "input-outline",
        "outline-dashed": "input-outline-dashed",
        filled: "input-filled",
        ghost: "input-ghost",
      },
      inputSize: {
        sm: "input-sm",
        default: "",
        lg: "input-lg",
      },
    },
    defaultVariants: {
      variant: "default",
      inputSize: "default",
    },
  },
)

export interface InputProps
  extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size">,
    VariantProps<typeof inputVariants> {
  leftIcon?: React.ReactNode
  rightIcon?: React.ReactNode
  error?: boolean
  errorMessage?: string
  clearable?: boolean
  onClear?: () => void
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      className,
      variant,
      inputSize,
      type = "text",
      leftIcon,
      rightIcon,
      error,
      errorMessage,
      clearable,
      onClear,
      ...props
    },
    ref,
  ) => {
    const [showPassword, setShowPassword] = React.useState(false)
    const isPassword = type === "password"
    const isSearch = type === "search"

    const actualType = isPassword ? (showPassword ? "text" : "password") : type

    const hasLeftIcon = Boolean(leftIcon || isSearch)
    const hasRightAdornment = Boolean(rightIcon || isPassword || (clearable && props.value))

    return (
      <div className="input-wrapper">
        {hasLeftIcon ? (
          <div className="input-icon input-icon-left">{leftIcon || (isSearch ? <Search /> : null)}</div>
        ) : null}

        <input
          ref={ref}
          type={actualType}
          className={cn(
            inputVariants({ variant, inputSize }),
            hasLeftIcon && "has-left-icon",
            hasRightAdornment && "has-right-icon",
            error && "input-error",
            className,
          )}
          {...props}
        />

        {hasRightAdornment ? (
          <div className="input-action-btn">
            {clearable && props.value ? (
              <Button type="button" variant="ghost" size="icon" onClick={onClear} className="h-5 w-5 p-0">
                <X className="h-4 w-4" />
              </Button>
            ) : null}
            {isPassword ? (
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => setShowPassword((prev) => !prev)}
                className="h-5 w-5 p-0"
              >
                {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </Button>
            ) : null}
            {rightIcon && !isPassword && !(clearable && props.value) ? rightIcon : null}
          </div>
        ) : null}

        {error && errorMessage ? <div className="input-error-msg">{errorMessage}</div> : null}
      </div>
    )
  },
)

Input.displayName = "Input"

export { Input, inputVariants }

import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"

import { cn } from "@/lib/utils"

const textareaVariants = cva(
  "flex w-full rounded-md transition-all duration-200 focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50",
  {
    variants: {
      variant: {
        default: "",
        outline: "textarea-outline",
        "outline-dashed": "textarea-outline-dashed",
        filled: "textarea-filled",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  },
)

export interface TextareaProps
  extends React.TextareaHTMLAttributes<HTMLTextAreaElement>,
    VariantProps<typeof textareaVariants> {
  error?: boolean
  autoResize?: boolean
  maxHeight?: number
  showCount?: boolean
}

const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  ({ className, variant, error, autoResize, maxHeight = 300, showCount, maxLength, ...props }, ref) => {
    const wrapperClassName = className
    const textareaRef = React.useRef<HTMLTextAreaElement | null>(null)

    const setRefs = React.useCallback(
      (node: HTMLTextAreaElement | null) => {
        textareaRef.current = node
        if (typeof ref === "function") {
          ref(node)
          return
        }
        if (ref) {
          ref.current = node
        }
      },
      [ref],
    )

    const resizeTextarea = React.useCallback(() => {
      if (!autoResize || !textareaRef.current) {
        return
      }
      textareaRef.current.style.height = "auto"
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, maxHeight)}px`
    }, [autoResize, maxHeight])

    React.useEffect(() => {
      resizeTextarea()
    }, [resizeTextarea, props.value])

    const handleInput = () => {
      resizeTextarea()
    }

    const rawValue = props.value
    const charCount = typeof rawValue === "string" ? rawValue.length : 0
    const overLimit = typeof maxLength === "number" && charCount > maxLength

    return (
      <div className={cn("textarea-wrapper", wrapperClassName)}>
        <textarea
          ref={setRefs}
          className={cn(textareaVariants({ variant }), error && "textarea-error", !autoResize && "no-resize")}
          maxLength={maxLength}
          {...props}
          onInput={(event) => {
            handleInput()
            props.onInput?.(event)
          }}
        />
        {showCount ? (
          <div className={cn("textarea-count", overLimit && "over-limit")}>
            {charCount}
            {typeof maxLength === "number" ? `/${maxLength}` : ""}
          </div>
        ) : null}
      </div>
    )
  },
)

Textarea.displayName = "Textarea"

export { Textarea, textareaVariants }

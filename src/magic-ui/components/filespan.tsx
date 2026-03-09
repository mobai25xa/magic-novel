import * as React from "react"

import { cn } from "@/lib/utils"

const filespanSizeMap = {
  sm: "filespan-sm",
  default: "",
  md: "filespan-md",
} as const

export interface FilespanProps extends React.HTMLAttributes<HTMLSpanElement> {
  path: string
  size?: keyof typeof filespanSizeMap
  icon?: React.ReactNode
  copyable?: boolean
  onCopy?: () => void
}

function splitPath(path: string) {
  return path.replace(/\\/g, "/").split("/").filter(Boolean)
}

const Filespan = React.forwardRef<HTMLSpanElement, FilespanProps>(
  (
    { path, size = "default", icon, copyable = false, onCopy, className, onClick, onKeyDown, ...props },
    ref
  ) => {
    const segments = React.useMemo(() => {
      const nextSegments = splitPath(path)
      return nextSegments.length > 0 ? nextSegments : [path]
    }, [path])

    const runCopy = React.useCallback(async () => {
      if (!copyable) {
        return
      }

      if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
        try {
          await navigator.clipboard.writeText(path)
        } catch {
          // noop
        }
      }

      onCopy?.()
    }, [copyable, onCopy, path])

    return (
      <span
        ref={ref}
        className={cn("filespan", filespanSizeMap[size], copyable && "copyable", className)}
        title={path}
        role={copyable ? "button" : props.role}
        tabIndex={copyable ? 0 : props.tabIndex}
        onClick={(event) => {
          onClick?.(event)
          if (!event.defaultPrevented) {
            void runCopy()
          }
        }}
        onKeyDown={(event) => {
          onKeyDown?.(event)
          if (!event.defaultPrevented && copyable && (event.key === "Enter" || event.key === " ")) {
            event.preventDefault()
            void runCopy()
          }
        }}
        {...props}
      >
        {icon ? <span className="filespan-icon">{icon}</span> : null}
        {segments.map((segment, index) => (
          <React.Fragment key={`${segment}-${index}`}>
            {index > 0 ? <span className="filespan-sep">/</span> : null}
            <span className={index === segments.length - 1 ? "filespan-last" : "filespan-seg"}>
              {segment}
            </span>
          </React.Fragment>
        ))}
      </span>
    )
  }
)
Filespan.displayName = "Filespan"

export { Filespan }

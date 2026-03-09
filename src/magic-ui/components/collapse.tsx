import * as React from "react"
import { ChevronDown } from "lucide-react"

import { cn } from "@/lib/utils"

export interface CollapseProps extends React.HTMLAttributes<HTMLDivElement> {
  defaultCollapsed?: boolean
  collapsed?: boolean
  onCollapsedChange?: (collapsed: boolean) => void
  maxHeight?: number
  label?: {
    expand: string
    collapse: string
  }
  children: React.ReactNode
}

const Collapse = React.forwardRef<HTMLDivElement, CollapseProps>(
  (
    {
      className,
      defaultCollapsed = true,
      collapsed,
      onCollapsedChange,
      maxHeight = 160,
      label,
      children,
      ...props
    },
    ref
  ) => {
    const contentRef = React.useRef<HTMLDivElement>(null)
    const [uncontrolledCollapsed, setUncontrolledCollapsed] = React.useState(defaultCollapsed)
    const [contentHeight, setContentHeight] = React.useState(0)
    const [overflowing, setOverflowing] = React.useState(false)

    const mergedLabel = React.useMemo(
      () => ({
        expand: label?.expand ?? "Show More",
        collapse: label?.collapse ?? "Show Less",
      }),
      [label]
    )

    const isControlled = collapsed !== undefined
    const isCollapsed = collapsed ?? uncontrolledCollapsed

    const syncState = React.useCallback(
      (nextCollapsed: boolean) => {
        if (!isControlled) {
          setUncontrolledCollapsed(nextCollapsed)
        }
        onCollapsedChange?.(nextCollapsed)
      },
      [isControlled, onCollapsedChange]
    )

    const measureHeight = React.useCallback(() => {
      const node = contentRef.current
      if (!node) {
        return
      }

      const nextHeight = node.scrollHeight
      setContentHeight(nextHeight)
      setOverflowing(nextHeight > maxHeight)
    }, [maxHeight])

    React.useEffect(() => {
      measureHeight()
    }, [children, maxHeight, measureHeight])

    React.useEffect(() => {
      if (!contentRef.current) {
        return
      }

      const observer = new ResizeObserver(() => measureHeight())
      observer.observe(contentRef.current)

      return () => observer.disconnect()
    }, [measureHeight])

    const resolvedMaxHeight = isCollapsed
      ? 0
      : (contentHeight || maxHeight)

    return (
      <div
        ref={ref}
        className={cn("collapse-block", className)}
        data-collapsed={isCollapsed}
        {...props}
      >
        <div
          ref={contentRef}
          className="collapse-content"
          style={{
            maxHeight: resolvedMaxHeight == null ? undefined : `${resolvedMaxHeight}px`,
          }}
        >
          {children}
        </div>

        {!isControlled && overflowing ? (
          <>
            <div className="collapse-gradient" />
            <button
              type="button"
              className="collapse-btn"
              aria-expanded={!isCollapsed}
              onClick={() => syncState(!isCollapsed)}
            >
              <span>{isCollapsed ? mergedLabel.expand : mergedLabel.collapse}</span>
              <ChevronDown className="collapse-arrow" />
            </button>
          </>
        ) : null}
      </div>
    )
  }
)
Collapse.displayName = "Collapse"

export { Collapse }

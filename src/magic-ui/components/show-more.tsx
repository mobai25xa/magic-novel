import * as React from "react"

import { cn } from "@/lib/utils"

export interface ShowMoreProps extends React.HTMLAttributes<HTMLDivElement> {
  maxLines?: number
  expandLabel?: string
  collapseLabel?: string
  children: React.ReactNode
}

const ShowMore = React.forwardRef<HTMLDivElement, ShowMoreProps>(
  (
    {
      className,
      maxLines = 3,
      expandLabel = "Show More",
      collapseLabel = "Show Less",
      children,
      ...props
    },
    ref
  ) => {
    const contentRef = React.useRef<HTMLDivElement>(null)
    const [expanded, setExpanded] = React.useState(false)
    const [overflowing, setOverflowing] = React.useState(false)

    const measureOverflow = React.useCallback(() => {
      const node = contentRef.current
      if (!node) {
        return
      }

      const prevDisplay = node.style.display
      const prevOverflow = node.style.overflow
      const prevLineClamp = node.style.getPropertyValue("-webkit-line-clamp")
      const prevBoxOrient = node.style.getPropertyValue("-webkit-box-orient")

      node.style.display = "-webkit-box"
      node.style.overflow = "hidden"
      node.style.setProperty("-webkit-line-clamp", `${maxLines}`)
      node.style.setProperty("-webkit-box-orient", "vertical")

      const isOverflow = node.scrollHeight > node.clientHeight + 1
      setOverflowing(isOverflow)

      node.style.display = prevDisplay
      node.style.overflow = prevOverflow
      if (prevLineClamp) {
        node.style.setProperty("-webkit-line-clamp", prevLineClamp)
      } else {
        node.style.removeProperty("-webkit-line-clamp")
      }
      if (prevBoxOrient) {
        node.style.setProperty("-webkit-box-orient", prevBoxOrient)
      } else {
        node.style.removeProperty("-webkit-box-orient")
      }
    }, [maxLines])

    React.useEffect(() => {
      measureOverflow()
    }, [children, maxLines, measureOverflow])

    React.useEffect(() => {
      if (!contentRef.current) {
        return
      }

      const observer = new ResizeObserver(() => measureOverflow())
      observer.observe(contentRef.current)

      return () => observer.disconnect()
    }, [measureOverflow])

    return (
      <div
        ref={ref}
        className={cn("show-more-block", expanded && "expanded", className)}
        {...props}
      >
        <div
          ref={contentRef}
          className="show-more-content"
          style={{ "--max-lines": `${maxLines}` } as React.CSSProperties}
        >
          {children}
        </div>

        {overflowing ? (
          <>
            <div className="show-more-fade" />
            <button
              type="button"
              className="show-more-btn"
              aria-expanded={expanded}
              onClick={() => setExpanded((prev) => !prev)}
            >
              {expanded ? collapseLabel : expandLabel}
            </button>
          </>
        ) : null}
      </div>
    )
  }
)
ShowMore.displayName = "ShowMore"

export { ShowMore }

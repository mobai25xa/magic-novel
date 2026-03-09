import { useEffect, useRef, useState, type ReactNode, type RefObject } from 'react'

type IntersectionDisplayProps = {
  id: string
  estimatedHeight?: number
  children: ReactNode
  onHeightChange?: (id: string, height: number) => void
  root?: RefObject<HTMLElement | null>
  rootMargin?: string
}

const DEFAULT_ROOT_MARGIN = '200px 0px'

export function IntersectionDisplay({
  id,
  estimatedHeight = 200,
  children,
  onHeightChange,
  root,
  rootMargin = DEFAULT_ROOT_MARGIN,
}: IntersectionDisplayProps) {
  const wrapperRef = useRef<HTMLDivElement>(null)
  const [visible, setVisible] = useState(false)
  const [lastMeasuredHeight, setLastMeasuredHeight] = useState(estimatedHeight)

  // Track visibility via IntersectionObserver
  useEffect(() => {
    const el = wrapperRef.current
    if (!el) return

    const observer = new IntersectionObserver(
      ([entry]) => {
        setVisible(entry.isIntersecting)
      },
      {
        root: root?.current ?? null,
        rootMargin,
      },
    )

    observer.observe(el)
    return () => observer.disconnect()
  }, [root, rootMargin])

  // Track height via ResizeObserver when visible
  useEffect(() => {
    if (!visible) return

    const el = wrapperRef.current
    if (!el) return

    const observer = new ResizeObserver(([entry]) => {
      const height = entry.contentRect.height
      if (height > 0 && Math.abs(height - lastMeasuredHeight) > 1) {
        setLastMeasuredHeight(height)
        onHeightChange?.(id, height)
      }
    })

    observer.observe(el)
    return () => observer.disconnect()
  }, [visible, id, onHeightChange, lastMeasuredHeight])

  return (
    <div
      ref={wrapperRef}
      data-turn-id={id}
      style={visible ? undefined : { minHeight: lastMeasuredHeight }}
    >
      {visible ? children : null}
    </div>
  )
}

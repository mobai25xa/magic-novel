import { useEffect, useRef } from 'react'

import { ContextMenu, ContextMenuContent, ContextMenuTrigger } from '@/magic-ui/components'

type CoordinateContextMenuProps = {
  x: number
  y: number
  onClose: () => void
  children: React.ReactNode
  contentClassName?: string
}

export function CoordinateContextMenu(input: CoordinateContextMenuProps) {
  const triggerRef = useRef<HTMLSpanElement | null>(null)

  useEffect(() => {
    const trigger = triggerRef.current
    if (!trigger) return

    const frame = requestAnimationFrame(() => {
      trigger.dispatchEvent(
        new MouseEvent('contextmenu', {
          bubbles: true,
          cancelable: true,
          clientX: input.x,
          clientY: input.y,
          button: 2,
          buttons: 2,
        }),
      )
    })

    return () => cancelAnimationFrame(frame)
  }, [input.x, input.y])

  return (
    <ContextMenu onOpenChange={(open) => !open && input.onClose()}>
      <ContextMenuTrigger
        ref={triggerRef}
        className="pointer-events-none fixed h-px w-px opacity-0"
        style={{ left: input.x, top: input.y }}
      />
      <ContextMenuContent className={input.contentClassName}>{input.children}</ContextMenuContent>
    </ContextMenu>
  )
}

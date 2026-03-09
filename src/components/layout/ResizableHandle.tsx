import { useEffect, useRef, useState } from 'react'

import { cn } from '@/lib/utils'

interface ResizableHandleProps {
  onResize: (delta: number) => void
  onResizeEnd?: () => void
  direction: 'left' | 'right'
}

export function ResizableHandle({ onResize, onResizeEnd, direction }: ResizableHandleProps) {
  const [isDragging, setIsDragging] = useState(false)
  const startXRef = useRef(0)
  const latestXRef = useRef(0)
  const rafIdRef = useRef<number | null>(null)

  useEffect(() => {
    if (!isDragging) return

    const flushResize = () => {
      rafIdRef.current = null
      const latestX = latestXRef.current
      const delta = direction === 'left' ? latestX - startXRef.current : startXRef.current - latestX
      if (delta === 0) return

      onResize(delta)
      startXRef.current = latestX
    }

    const handleMouseMove = (event: MouseEvent) => {
      latestXRef.current = event.clientX
      if (rafIdRef.current !== null) return
      rafIdRef.current = requestAnimationFrame(flushResize)
    }

    const handleMouseUp = () => {
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current)
        rafIdRef.current = null
      }
      flushResize()
      onResizeEnd?.()
      setIsDragging(false)
    }

    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)

    return () => {
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      if (rafIdRef.current !== null) {
        cancelAnimationFrame(rafIdRef.current)
        rafIdRef.current = null
      }
    }
  }, [direction, isDragging, onResize, onResizeEnd])

  const handleMouseDown = (event: React.MouseEvent) => {
    event.preventDefault()
    startXRef.current = event.clientX
    latestXRef.current = event.clientX
    setIsDragging(true)
  }

  return (
    <div
      onMouseDown={handleMouseDown}
      className={cn(
        'resizable-handle group',
        direction === 'left' ? 'resizable-handle-left' : 'resizable-handle-right',
        isDragging && 'resizable-handle-active',
      )}
    >
      <div className="absolute inset-0 -left-1 -right-1" />
      <div className="resizable-handle-bar" />
    </div>
  )
}

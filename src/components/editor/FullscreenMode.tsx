/**
 * @author Gamma
 * @date 2026-02-11
 * @description 全屏/沉浸写作模式
 */
import { useEffect } from 'react'
import { useLayoutStore } from '@/stores/layout-store'
import { eventBus, EVENTS } from '@/lib/events'

interface FullscreenModeProps {
  children: React.ReactNode
}

export function FullscreenMode({ children }: FullscreenModeProps) {
  const { isFullscreen, toggleFullscreen } = useLayoutStore()

  // 监听 F11 事件（由 Alpha 的快捷键扩展发射）
  useEffect(() => {
    const handleToggle = () => toggleFullscreen()
    eventBus.on(EVENTS.FULLSCREEN_TOGGLE, handleToggle)
    return () => eventBus.off(EVENTS.FULLSCREEN_TOGGLE, handleToggle)
  }, [toggleFullscreen])

  // 监听 Esc 退出全屏
  useEffect(() => {
    if (!isFullscreen) return

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        toggleFullscreen()
      }
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [isFullscreen, toggleFullscreen])

  if (!isFullscreen) {
    return <>{children}</>
  }

  return (
    <div className="fullscreen-overlay">
      <div className="flex-1 overflow-hidden">
        {children}
      </div>
    </div>
  )
}

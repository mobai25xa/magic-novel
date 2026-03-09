import { useState, useEffect, useRef } from 'react'
import { ChevronUp, ChevronDown } from 'lucide-react'
import { useTranslation } from '@/hooks/use-translation'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/magic-ui/components'

const HIDE_DELAY_MS = 220

interface ScrollButtonsProps {
  containerRef: React.RefObject<HTMLDivElement>
}

function ScrollFloatingButton(input: { onClick: () => void; title: string; children: React.ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={input.onClick}
          aria-label={input.title}
          className="scroll-fab"
        >
          {input.children}
        </button>
      </TooltipTrigger>
      <TooltipContent>{input.title}</TooltipContent>
    </Tooltip>
  )
}

function useScrollButtonVisibility(containerRef: React.RefObject<HTMLDivElement>) {
  const [showUp, setShowUp] = useState(false)
  const [showDown, setShowDown] = useState(false)
  const [isActive, setIsActive] = useState(false)
  const [isButtonsHovered, setIsButtonsHovered] = useState(false)
  const hideTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const isButtonsHoveredRef = useRef(false)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    const clearHideTimer = () => {
      if (hideTimeoutRef.current) {
        clearTimeout(hideTimeoutRef.current)
        hideTimeoutRef.current = null
      }
    }

    const updateScrollButtons = () => {
      const { scrollTop, scrollHeight, clientHeight } = container
      const isAtTop = scrollTop < 5
      const isAtBottom = scrollTop + clientHeight >= scrollHeight - 5

      setShowUp(!isAtTop)
      setShowDown(!isAtBottom)
    }

    const showButtons = () => {
      setIsActive(true)
      clearHideTimer()
    }

    const scheduleHide = () => {
      clearHideTimer()
      hideTimeoutRef.current = setTimeout(() => {
        if (!isButtonsHoveredRef.current) {
          setIsActive(false)
        }
      }, HIDE_DELAY_MS)
    }

    const handleScroll = () => {
      updateScrollButtons()
      showButtons()
    }

    const handleMouseEnter = () => {
      showButtons()
    }

    const handleMouseLeave = () => {
      scheduleHide()
    }

    const handlePointerDown = () => {
      showButtons()
    }

    const handleWindowBlur = () => {
      setIsActive(false)
      setIsButtonsHovered(false)
      isButtonsHoveredRef.current = false
      clearHideTimer()
    }

    container.addEventListener('scroll', handleScroll)
    container.addEventListener('mouseenter', handleMouseEnter)
    container.addEventListener('mouseleave', handleMouseLeave)
    container.addEventListener('pointerdown', handlePointerDown)
    window.addEventListener('blur', handleWindowBlur)

    updateScrollButtons()

    return () => {
      container.removeEventListener('scroll', handleScroll)
      container.removeEventListener('mouseenter', handleMouseEnter)
      container.removeEventListener('mouseleave', handleMouseLeave)
      container.removeEventListener('pointerdown', handlePointerDown)
      window.removeEventListener('blur', handleWindowBlur)
      clearHideTimer()
    }
  }, [containerRef])

  const handleButtonsMouseEnter = () => {
    isButtonsHoveredRef.current = true
    setIsButtonsHovered(true)
    if (hideTimeoutRef.current) {
      clearTimeout(hideTimeoutRef.current)
      hideTimeoutRef.current = null
    }
    setIsActive(true)
  }

  const handleButtonsMouseLeave = () => {
    isButtonsHoveredRef.current = false
    setIsButtonsHovered(false)
    hideTimeoutRef.current = setTimeout(() => {
      setIsActive(false)
    }, HIDE_DELAY_MS)
  }

  return {
    showUp,
    showDown,
    isVisible: (isActive || isButtonsHovered) && (showUp || showDown),
    handleButtonsMouseEnter,
    handleButtonsMouseLeave,
  }
}

export function ScrollButtons({ containerRef }: ScrollButtonsProps) {
  const { translations } = useTranslation()
  const { showUp, showDown, isVisible, handleButtonsMouseEnter, handleButtonsMouseLeave } = useScrollButtonVisibility(containerRef)

  if (!isVisible) return null

  return (
    <TooltipProvider>
      <div
        className="scroll-fab-stack absolute right-6 bottom-6 flex flex-col gap-2 z-50"
        onMouseEnter={handleButtonsMouseEnter}
        onMouseLeave={handleButtonsMouseLeave}
      >
        {showUp && (
          <ScrollFloatingButton
            onClick={() => containerRef.current?.scrollTo({ top: 0, behavior: 'smooth' })}
            title={translations.editor.scrollToTop}
          >
            <ChevronUp className="h-4 w-4" />
          </ScrollFloatingButton>
        )}

        {showDown && (
          <ScrollFloatingButton
            onClick={() => {
              const container = containerRef.current
              if (container) {
                container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' })
              }
            }}
            title={translations.editor.scrollToBottom}
          >
            <ChevronDown className="h-4 w-4" />
          </ScrollFloatingButton>
        )}
      </div>
    </TooltipProvider>
  )
}

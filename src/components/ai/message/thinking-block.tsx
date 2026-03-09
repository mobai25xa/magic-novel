import { useEffect, useRef } from 'react'
import { Brain, ChevronDown } from 'lucide-react'

import { useAiTranslations } from '../ai-hooks'

type ThinkingBlockProps = {
  text: string
  streaming?: boolean
  defaultOpen?: boolean
}

export function ThinkingBlock({ text, streaming, defaultOpen }: ThinkingBlockProps) {
  const ai = useAiTranslations()
  const detailsRef = useRef<HTMLDetailsElement>(null)

  // Auto-open while streaming, auto-close when done
  useEffect(() => {
    if (!detailsRef.current) return

    if (streaming) {
      detailsRef.current.open = true
    } else if (!defaultOpen) {
      detailsRef.current.open = false
    }
  }, [streaming, defaultOpen])

  if (!text) {
    return null
  }

  return (
    <details
      ref={detailsRef}
      open={streaming || defaultOpen}
      className="ai-thinking ai-animate-fade-in"
    >
      <summary className="ai-thinking-header">
        <span className="thinking-icon" aria-hidden="true">
          <Brain className="h-2.5 w-2.5" />
        </span>
        <span>{ai.turn.thinking}</span>
        <span className="thinking-chevron" aria-hidden="true">
          <ChevronDown className="h-3.5 w-3.5" />
        </span>
      </summary>

      <div className="ai-thinking-body">
        <div className="ai-thinking-content">
          {ai.turn.thinkingPlaceholder}
        </div>
      </div>
    </details>
  )
}

import { useEffect, useState } from 'react'

import { Bot } from 'lucide-react'

import { Skeleton } from '@/magic-ui/components'
import type { FeedbackRating } from '@/lib/agent-chat/types'

import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'
import { MarkdownRenderer } from '../markdown/MarkdownRenderer'
import { MessageActions } from './message-actions'
import { MessageTimestamp } from './message-timestamp'
import { FeedbackPanel } from './feedback-panel'

type TurnCardAssistantBlockProps = {
  assistantText: string
  elapsedLabel: string
  loading?: boolean
  streaming?: boolean
  timestamp?: number
  turnId: number
  feedbackRating: FeedbackRating
  onRate: (rating: FeedbackRating) => void
  retryable?: boolean
  onRetry?: () => void
  label?: string
  showElapsedLabel?: boolean
  showFooterActions?: boolean
}

export function TurnCardAssistantBlock(input: TurnCardAssistantBlockProps) {
  const ai = useAiTranslations()
  const [isFirst, setIsFirst] = useState(true)

  useEffect(() => {
    queueMicrotask(() => {
      setIsFirst(false)
    })
  }, [])

  const hasText = Boolean(input.assistantText.trim())
  const assistantLabel = input.label || ai.turn.assistant
  const showElapsedLabel = input.showElapsedLabel !== false
  const showFooterActions = input.showFooterActions !== false

  return (
    <div className={cn('group/assistant editor-shell-ai-assistant-group px-0.5', isFirst && 'ai-animate-fly-in')}>
      <div className="editor-shell-ai-response-header">
        <span className={cn('editor-shell-ai-response-dot', input.streaming && 'editor-shell-ai-response-dot-streaming')} aria-hidden="true" />
        <Bot className="h-3.5 w-3.5 text-[var(--text-success-dark)]" aria-hidden="true" />
        <span className="editor-shell-ai-response-label">{assistantLabel}</span>
        {showElapsedLabel ? (
          input.timestamp ? (
            <MessageTimestamp timestamp={input.timestamp} className="editor-shell-ai-response-time" />
          ) : (
            <span className="editor-shell-ai-response-time">{input.elapsedLabel}</span>
          )
        ) : null}
      </div>

      <div className="editor-shell-ai-assistant-text text-sm leading-relaxed break-words min-w-0 max-w-full overflow-visible py-0.5">
        {hasText ? (
          <MarkdownRenderer text={input.assistantText} streaming={input.streaming} className="overflow-visible max-w-none" />
        ) : (
          <span className="whitespace-pre-wrap text-foreground">{input.assistantText}</span>
        )}

        {input.loading && !hasText ? (
          <div className="mt-2 space-y-2.5">
            <Skeleton className="h-4 w-[85%]" />
            <Skeleton className="h-4 w-[70%]" />
            <Skeleton className="h-4 w-[55%]" />
          </div>
        ) : null}
      </div>

      {hasText && showFooterActions ? (
        <div className="mt-2 flex items-end justify-between gap-2">
          <FeedbackPanel
            turnId={input.turnId}
            rating={input.feedbackRating}
            streaming={input.streaming}
            onRate={input.onRate}
          />
          <div className="flex items-center gap-2">
            <MessageActions
              text={input.assistantText}
              retryable={input.retryable}
              streaming={input.streaming}
              onRetry={input.onRetry}
            />
          </div>
        </div>
      ) : null}
    </div>
  )
}

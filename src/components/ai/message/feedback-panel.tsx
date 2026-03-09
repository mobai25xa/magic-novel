import { useCallback, useState } from 'react'
import { ThumbsUp, ThumbsDown, MoreHorizontal } from 'lucide-react'

import { Button } from '@/magic-ui/components'
import { cn } from '@/lib/utils'
import type { FeedbackRating } from '@/lib/agent-chat/types'

import { useAiTranslations } from '../ai-hooks'

type FeedbackPanelProps = {
  turnId: number
  rating: FeedbackRating
  streaming?: boolean
  onRate: (rating: FeedbackRating) => void
  className?: string
}

export function FeedbackPanel({ turnId: _turnId, rating, streaming, onRate, className }: FeedbackPanelProps) {
  const ai = useAiTranslations()
  const [justRated, setJustRated] = useState(false)

  const handlePositive = useCallback(() => {
    onRate(rating === 'positive' ? 'unset' : 'positive')
    setJustRated(true)
    setTimeout(() => setJustRated(false), 300)
  }, [rating, onRate])

  const handleNegative = useCallback(() => {
    onRate(rating === 'negative' ? 'unset' : 'negative')
    setJustRated(true)
    setTimeout(() => setJustRated(false), 300)
  }, [rating, onRate])

  if (streaming) {
    return null
  }

  const showPositive = rating !== 'negative'
  const showNegative = rating !== 'positive'

  return (
    <div
      className={cn(
        'editor-shell-ai-feedback-actions flex items-center gap-1',
        rating === 'unset' && 'opacity-0 transition-opacity group-hover/assistant:opacity-100 duration-[var(--ai-duration-fast)]',
        className,
      )}
    >
      {showPositive ? (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={handlePositive}
          aria-label={ai.action.feedbackLike}
          aria-pressed={rating === 'positive'}
          className={cn(
            'editor-shell-ai-action-btn h-7 w-7',
            rating === 'positive'
              ? 'text-[var(--ai-feedback-positive)]'
              : 'text-muted-foreground',
            justRated && rating === 'positive' && 'ai-animate-scale-in',
          )}
        >
          <ThumbsUp className={cn('h-3.5 w-3.5', rating === 'positive' && 'fill-current')} />
        </Button>
      ) : null}

      {showNegative ? (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={handleNegative}
          aria-label={ai.action.feedbackDislike}
          aria-pressed={rating === 'negative'}
          className={cn(
            'editor-shell-ai-action-btn h-7 w-7',
            rating === 'negative'
              ? 'text-[var(--ai-feedback-negative)]'
              : 'text-muted-foreground',
            justRated && rating === 'negative' && 'ai-animate-scale-in',
          )}
        >
          <ThumbsDown className={cn('h-3.5 w-3.5', rating === 'negative' && 'fill-current')} />
        </Button>
      ) : null}

      {rating === 'unset' ? (
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={ai.action.submitFeedback}
          className="editor-shell-ai-action-btn h-7 w-7"
        >
          <MoreHorizontal className="h-3.5 w-3.5" />
        </Button>
      ) : null}
    </div>
  )
}

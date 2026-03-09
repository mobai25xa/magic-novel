import { useCallback } from 'react'

import type { AgentUiTurnState, FeedbackRating } from '@/lib/agent-chat/types'
import { useAgentChatStore } from '@/state/agent'

import { useAiTranslations } from '../ai-hooks'
import { useTurnCardContentModel } from './turn-card-content-hooks'
import { TurnCardAssistantBlock } from './turn-card-assistant-block'

type TurnCardContentProps = {
  text: string
  turn: AgentUiTurnState
  running: boolean
  retryable?: boolean
  onRetry?: () => void
  hideInlineLoadingIndicator?: boolean
}

function pad2(value: number) {
  return String(value).padStart(2, '0')
}

function formatElapsedLabel(elapsedMs: number) {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1000))

  if (totalSeconds < 60) {
    return `${totalSeconds}s`
  }

  if (totalSeconds < 3600) {
    const minutes = Math.floor(totalSeconds / 60)
    const seconds = totalSeconds % 60
    return `${minutes}:${pad2(seconds)}`
  }

  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  return `${hours}:${pad2(minutes)}:${pad2(seconds)}`
}

function isActiveTurn(phase: AgentUiTurnState['phase']) {
  return phase === 'queued'
    || phase === 'planning'
    || phase === 'tool_running'
    || phase === 'synthesizing'
    || phase === 'compacting'
}

export function TurnCardContent(input: TurnCardContentProps) {
  const ai = useAiTranslations()

  const feedbackRating = useAgentChatStore(
    (s) => s.turnFeedback[input.turn.turn] ?? 'unset',
  ) as FeedbackRating
  const setTurnFeedback = useAgentChatStore((s) => s.setTurnFeedback)

  const handleRate = useCallback(
    (rating: FeedbackRating) => {
      setTurnFeedback(input.turn.turn, rating)
    },
    [input.turn.turn, setTurnFeedback],
  )

  const model = useTurnCardContentModel({
    text: input.text,
    turn: input.turn,
    running: input.running,
  })

  const active = isActiveTurn(input.turn.phase)
  const showContinuousLoading = active && input.running
  const loading = input.hideInlineLoadingIndicator ? false : showContinuousLoading
  const elapsedLabel = formatElapsedLabel(model.elapsedMs)
  const assistantText = !input.running
    ? (model.rawHasAnswer ? input.text : ai.turn.assistantPlaceholder)
    : model.hasAnswer
      ? model.typedAnswer
      : model.rawHasAnswer
        ? input.text
        : ai.turn.assistantPlaceholder

  return (
    <TurnCardAssistantBlock
      assistantText={assistantText}
      elapsedLabel={elapsedLabel}
      loading={loading}
      streaming={model.isStreaming}
      timestamp={undefined}
      turnId={input.turn.turn}
      feedbackRating={feedbackRating}
      onRate={handleRate}
      retryable={input.retryable}
      onRetry={input.onRetry}
    />
  )
}

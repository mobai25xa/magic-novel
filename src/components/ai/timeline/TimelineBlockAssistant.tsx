import type { TimelineBlock } from '@/lib/agent-chat/timeline'
import type { AgentUiTurnState } from '@/lib/agent-chat/types'

import { TurnCardContent } from '../message/turn-card-content'

type TimelineBlockAssistantProps = {
  block: Extract<TimelineBlock, { type: 'assistant_segment' }>
  turn: AgentUiTurnState
  running: boolean
  hideInlineLoadingIndicator: boolean
  onRetryTurn?: () => void
}

export function TimelineBlockAssistant(input: TimelineBlockAssistantProps) {
  return (
    <TurnCardContent
      text={input.block.text}
      turn={input.turn}
      running={input.running}
      retryable={!input.running && input.turn.phase === 'failed'}
      hideInlineLoadingIndicator={input.hideInlineLoadingIndicator}
      onRetry={input.onRetryTurn}
    />
  )
}

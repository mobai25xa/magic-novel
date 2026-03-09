import type { AgentAskUserAnswer, AgentPendingAskUserRequest } from '@/agent/types'
import type { TimelineBlock } from '@/lib/agent-chat/timeline'
import type {
  AgentUiToolStep,
  AgentUiTurnState,
} from '@/lib/agent-chat/types'
import type { AiChatViewMode } from '@/state/settings'

import { TimelineBlockAssistant } from './TimelineBlockAssistant'
import { TimelineBlockThinkingPanel } from './TimelineBlockThinkingPanel'
import { TimelineBlockToolCall } from './TimelineBlockToolCall'

type TimelineBlocksRendererProps = {
  blocks: TimelineBlock[]
  turn: AgentUiTurnState
  toolSteps: AgentUiToolStep[]
  sessionId: string
  running: boolean
  viewMode: AiChatViewMode
  onRetryStep: (turnId: number, callId: string) => void
  onRetryTurn?: () => void
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
  pendingAskUser?: AgentPendingAskUserRequest
  onResolveAskUser: (callId: string, answers: AgentAskUserAnswer[]) => void
  onCancelAskUser: (callId: string) => void
  hideInlineLoadingIndicator: boolean
}

export function TimelineBlocksRenderer(input: TimelineBlocksRendererProps) {
  const stepByCallId = new Map(input.toolSteps.map((step) => [step.callId, step] as const))

  return (
    <>
      {input.blocks.map((block) => {
        if (block.type === 'assistant_segment') {
          return (
            <TimelineBlockAssistant
              key={block.id}
              block={block}
              turn={input.turn}
              running={input.running}
              hideInlineLoadingIndicator={input.hideInlineLoadingIndicator}
              onRetryTurn={input.onRetryTurn}
            />
          )
        }

        if (block.type === 'thinking_panel') {
          return (
            <TimelineBlockThinkingPanel
              key={block.id}
              block={block}
              running={input.running}
            />
          )
        }

        return (
          <TimelineBlockToolCall
            key={block.id}
            block={block}
            step={stepByCallId.get(block.callId)}
            turnId={input.turn.turn}
            sessionId={input.sessionId}
            running={input.running}
            viewMode={input.viewMode}
            onRetryStep={input.onRetryStep}
            onApprove={input.onApprove}
            onSkip={input.onSkip}
            pendingAskUser={input.pendingAskUser}
            onResolveAskUser={input.onResolveAskUser}
            onCancelAskUser={input.onCancelAskUser}
          />
        )
      })}
    </>
  )
}

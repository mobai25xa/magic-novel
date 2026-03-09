import type { AgentAskUserAnswer, AgentPendingAskUserRequest } from '@/agent/types'
import { isAskUserToolName } from '@/lib/agent-chat/askuser'
import type { TimelineBlock } from '@/lib/agent-chat/timeline'
import type { AgentUiToolStep } from '@/lib/agent-chat/types'
import type { AiChatViewMode } from '@/state/settings'

import { AskUserInlineCard } from '../askuser/AskUserInlineCard'
import { ToolTimeline } from '../tool-timeline'

type TimelineBlockToolCallProps = {
  block: Extract<TimelineBlock, { type: 'tool_call' }>
  step?: AgentUiToolStep
  turnId: number
  sessionId: string
  running: boolean
  viewMode: AiChatViewMode
  onRetryStep: (turnId: number, callId: string) => void
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
  pendingAskUser?: AgentPendingAskUserRequest
  onResolveAskUser: (callId: string, answers: AgentAskUserAnswer[]) => void
  onCancelAskUser: (callId: string) => void
}

export function TimelineBlockToolCall(input: TimelineBlockToolCallProps) {
  if (!input.step) {
    return null
  }

  const isAskUserStep = isAskUserToolName(input.step.toolName)
  const shouldRenderAskUserCard = Boolean(
    isAskUserStep
      && input.step.progress === 'waiting_askuser'
      && input.pendingAskUser
      && input.pendingAskUser.turn === input.turnId
      && input.pendingAskUser.callId === input.step.callId,
  )

  if (shouldRenderAskUserCard) {
    return (
      <AskUserInlineCard
        request={input.pendingAskUser}
        onSubmit={input.onResolveAskUser}
        onCancel={input.onCancelAskUser}
      />
    )
  }

  return (
    <ToolTimeline
      steps={[input.step]}
      turnId={input.turnId}
      sessionId={input.sessionId}
      running={input.running}
      onRetryStep={input.onRetryStep}
      viewMode={input.viewMode}
      onApprove={input.onApprove}
      onSkip={input.onSkip}
    />
  )
}

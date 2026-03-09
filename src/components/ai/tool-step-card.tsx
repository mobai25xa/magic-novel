import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { useStepRenderMetric } from './turn-metrics'
import { useWaitingConfirmationMetric } from './tool/tool-call-hooks'
import { ToolCallCard } from './tool/ToolCallCard'

type ToolStepCardProps = {
  step: AgentUiToolStep
  turnId?: number
  sessionId: string
  running: boolean
  onRetryStep: (turnId: number, callId: string) => void
  viewMode?: 'compact' | 'debug'
  isLastAwaitingApproval?: boolean
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
}

export function ToolStepCard(input: ToolStepCardProps) {
  useStepRenderMetric({
    sessionId: input.sessionId,
    turnId: input.turnId,
    callId: input.step.callId,
    status: input.step.status,
  })

  const stepRunning = input.step.status === 'running' || input.step.status === 'waiting_confirmation'
  useWaitingConfirmationMetric({
    sessionId: input.sessionId,
    turnId: input.turnId,
    step: input.step,
    running: stepRunning,
  })

  if (typeof input.turnId !== 'number') {
    return null
  }

  return (
    <ToolCallCard
      step={input.step}
      turnId={input.turnId}
      sessionId={input.sessionId}
      running={input.running}
      viewMode={input.viewMode}
      isLastAwaitingApproval={input.isLastAwaitingApproval}
      onRetryStep={input.onRetryStep}
      onApprove={input.onApprove}
      onSkip={input.onSkip}
    />
  )
}

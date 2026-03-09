import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { ToolTimelineLayout } from './message/tool-timeline-layout'

type ToolTimelineProps = {
  steps: AgentUiToolStep[]
  turnId?: number
  sessionId: string
  running: boolean
  onRetryStep: (turnId: number, callId: string) => void
  viewMode?: 'compact' | 'debug'
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
}

export function ToolTimeline(input: ToolTimelineProps) {
  if (input.steps.length === 0) {
    return null
  }

  return (
    <ToolTimelineLayout
      filtered={input.steps}
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

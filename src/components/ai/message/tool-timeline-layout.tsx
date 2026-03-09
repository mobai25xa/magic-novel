import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { ToolStepCard } from '../tool-step-card'

type ToolTimelineLayoutProps = {
  filtered: AgentUiToolStep[]
  turnId?: number
  sessionId: string
  running: boolean
  onRetryStep: (turnId: number, callId: string) => void
  viewMode?: 'compact' | 'debug'
  onApprove?: (callId: string) => void
  onSkip?: (callId: string) => void
}

export function ToolTimelineLayout(input: ToolTimelineLayoutProps) {
  const lastAwaitingCallId = useMemo(() => {
    const awaiting = input.filtered.filter((s) => s.status === 'waiting_confirmation' && s.progress === 'waiting_confirmation')
    return awaiting.length > 0 ? awaiting[awaiting.length - 1].callId : null
  }, [input.filtered])

  return (
    <div className="space-y-1.5 pl-2">
      {input.filtered.map((step) => (
        <ToolStepCard
          key={step.callId}
          step={step}
          turnId={input.turnId}
          sessionId={input.sessionId}
          running={input.running}
          onRetryStep={input.onRetryStep}
          viewMode={input.viewMode}
          isLastAwaitingApproval={step.callId === lastAwaitingCallId}
          onApprove={input.onApprove}
          onSkip={input.onSkip}
        />
      ))}
    </div>
  )
}

import { useEffect, useState } from 'react'

import { logUiMetric } from '@/agent/telemetry'
import type { AgentUiToolStep } from '@/lib/agent-chat/types'

export function useRunningStepClock(running: boolean) {
  const [now, setNow] = useState(() => Date.now())

  useEffect(() => {
    if (!running) {
      return
    }

    const id = window.setInterval(() => setNow(Date.now()), 250)
    return () => window.clearInterval(id)
  }, [running])

  return now
}

export function useWaitingConfirmationMetric(input: {
  sessionId: string
  turnId?: number
  step: AgentUiToolStep
  running: boolean
}) {
  useEffect(() => {
    if (!input.running || !input.turnId) {
      return
    }

    const startedAt = performance.now()
    return () => {
      logUiMetric({
        sessionId: input.sessionId,
        turnId: input.turnId,
        metric: 'waiting_for_tool_confirmation_ms',
        value: performance.now() - startedAt,
        tags: {
          callId: input.step.callId,
          toolName: input.step.toolName,
          status: input.step.status,
        },
      })
    }
  }, [
    input.running,
    input.sessionId,
    input.step.callId,
    input.step.status,
    input.step.toolName,
    input.turnId,
  ])
}

import { useEffect } from 'react'

import { logUiMetric } from '@/agent/telemetry'

export function useTurnRenderMetric(input: {
  sessionId: string
  turnId: number
  stepCount: number
}) {
  useEffect(() => {
    const startedAt = performance.now()
    return () => {
      logUiMetric({
        sessionId: input.sessionId,
        turnId: input.turnId,
        metric: 'turn_render_ms',
        value: performance.now() - startedAt,
        tags: {
          stepCount: input.stepCount,
        },
      })
    }
  }, [input.sessionId, input.stepCount, input.turnId])
}

export function useStepRenderMetric(input: {
  sessionId: string
  turnId?: number
  callId: string
  status: string
}) {
  useEffect(() => {
    const startedAt = performance.now()
    return () => {
      logUiMetric({
        sessionId: input.sessionId,
        turnId: input.turnId,
        metric: 'step_render_ms',
        value: performance.now() - startedAt,
        tags: {
          callId: input.callId,
          status: input.status,
        },
      })
    }
  }, [input.callId, input.sessionId, input.status, input.turnId])
}

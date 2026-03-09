import { useAgentTelemetryStore } from './store'

export function logConcurrency(event: { sessionId: string; queued: number; running: number }) {
  useAgentTelemetryStore.getState().addConcurrency({
    session_id: event.sessionId,
    queued: event.queued,
    running: event.running,
    ts: Date.now(),
  })
}

export function logUiMetric(event: {
  sessionId: string
  metric: string
  value: number
  turnId?: number
  tags?: Record<string, string | number | boolean>
}) {
  useAgentTelemetryStore.getState().addUiMetric({
    session_id: event.sessionId,
    turn_id: event.turnId,
    metric: event.metric,
    value: event.value,
    tags: event.tags,
    ts: Date.now(),
  })
}

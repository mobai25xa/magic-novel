export interface AgentConcurrencyEvent {
  session_id: string
  queued: number
  running: number
  ts: number
}

export interface AgentUiMetricEvent {
  session_id: string
  turn_id?: number
  metric: string
  value: number
  tags?: Record<string, string | number | boolean>
  ts: number
}

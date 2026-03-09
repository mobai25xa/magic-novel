import { create } from 'zustand'

import type {
  AgentConcurrencyEvent,
  AgentUiMetricEvent,
} from './types'

const MAX_EVENTS = 200

interface AgentTelemetryState {
  concurrency: AgentConcurrencyEvent[]
  uiMetrics: AgentUiMetricEvent[]

  addConcurrency: (event: AgentConcurrencyEvent) => void
  addUiMetric: (event: AgentUiMetricEvent) => void
  reset: () => void
}

function pushWithLimit<T>(items: T[], item: T): T[] {
  const next = [...items, item]
  if (next.length <= MAX_EVENTS) return next
  return next.slice(next.length - MAX_EVENTS)
}

export const useAgentTelemetryStore = create<AgentTelemetryState>((set) => ({
  concurrency: [],
  uiMetrics: [],

  addConcurrency: (event) =>
    set((state) => ({ concurrency: pushWithLimit(state.concurrency, event) })),

  addUiMetric: (event) =>
    set((state) => ({ uiMetrics: pushWithLimit(state.uiMetrics, event) })),

  reset: () =>
    set({
      concurrency: [],
      uiMetrics: [],
    }),
}))

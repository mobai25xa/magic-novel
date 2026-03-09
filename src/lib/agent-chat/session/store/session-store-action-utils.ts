import { formatUnknownError } from '@/lib/error-utils'
import { logUiMetric } from '@/agent/telemetry'

import type {
  GetStoreState,
  SetStoreState,
} from './session-store-types'

export function unknownErrorToString(error: unknown) {
  return formatUnknownError(error)
}

export function logSessionMetric(input: {
  get: GetStoreState
  metric: string
  value?: number
  tags?: Record<string, string | number | boolean>
  sessionId?: string
}) {
  const resolvedSessionId = input.sessionId?.trim()
    || input.get().currentSessionMeta?.session_id
    || input.get().session_id
    || 'session_unknown'

  logUiMetric({
    sessionId: resolvedSessionId,
    metric: input.metric,
    value: typeof input.value === 'number' ? input.value : 1,
    tags: input.tags,
  })
}

export function createStateHelpers(set: SetStoreState, get: GetStoreState) {
  return {
    setLoading(value: boolean) {
      set({ isSessionLoading: value })
    },
    setError(value: string | null) {
      set({
        sessionError: value,
        pendingSessionReminder: value ? undefined : get().pendingSessionReminder,
      })
    },
    getCurrentSessionId() {
      return get().currentSessionMeta?.session_id
    },
  }
}

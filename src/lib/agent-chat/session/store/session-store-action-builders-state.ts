import type { AgentSessionReminder } from '@/agent/types'

import type { GetStoreState, SetStoreState } from './session-store-types'

export function createConsumeResumedAction(set: SetStoreState, get: GetStoreState) {
  return () => {
    const resumed = get().wasSessionResumed
    if (resumed) {
      set({ wasSessionResumed: false })
    }
    return resumed
  }
}

export function createConsumeSessionReminderAction(set: SetStoreState, get: GetStoreState) {
  return (): AgentSessionReminder | undefined => {
    const reminder = get().pendingSessionReminder
    if (reminder) {
      set({ pendingSessionReminder: undefined })
    }
    return reminder
  }
}

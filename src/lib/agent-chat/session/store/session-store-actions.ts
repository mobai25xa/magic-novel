import type { SessionPersistenceStoreActions } from './session-store-contract'
import {
  createApplySessionEventsAction,
  createApplySessionHydrationAction,
} from './session-store-action-builders'
import {
  createDeleteSessionAction,
  createEnsureSessionAction,
  createLoadSessionListAction,
  createRenameSessionAction,
  createResumeSessionAction,
  createStartSessionAction,
} from './session-store-action-builders-persist'
import {
  createConsumeResumedAction,
  createConsumeSessionReminderAction,
} from './session-store-action-builders-state'
import type { GetStoreState, SetStoreState } from './session-store-types'

export function createSessionPersistenceActions(input: {
  set: SetStoreState
  get: GetStoreState
}): SessionPersistenceStoreActions {
  const applySessionEvents = createApplySessionEventsAction(input.set)
  const applySessionHydration = createApplySessionHydrationAction(input.set)

  return {
    ensurePersistedSession: createEnsureSessionAction({
      set: input.set,
      get: input.get,
    }),
    startNewPersistedSession: createStartSessionAction({
      set: input.set,
      get: input.get,
      applySessionEvents,
    }),
    loadPersistedSessionList: createLoadSessionListAction(input.set, input.get),
    resumePersistedSession: createResumeSessionAction({
      set: input.set,
      get: input.get,
      applySessionEvents,
      applySessionHydration,
    }),
    renamePersistedSession: createRenameSessionAction(input.set, input.get),
    deletePersistedSession: createDeleteSessionAction(input.set, input.get),
    consumeWasSessionResumed: createConsumeResumedAction(input.set, input.get),
    consumeSessionReminder: createConsumeSessionReminderAction(input.set, input.get),
    applySessionEvents,
    applySessionHydration,
  }
}

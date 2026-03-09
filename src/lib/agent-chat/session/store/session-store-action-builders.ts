import { reduceSessionEventsToStore } from '../session-reducer'
import type {
  ApplySessionEvents,
  ApplySessionHydration,
  SetStoreState,
} from './session-store-types'

export function createApplySessionEventsAction(set: SetStoreState): ApplySessionEvents {
  return (input) => {
    const patch = reduceSessionEventsToStore({
      sessionId: input.sessionId,
      events: input.events,
      meta: input.meta,
      replayedAt: input.replayedAt,
    })

    set({
      ...patch,
      currentSessionMeta: input.meta,
      stateStatus: 'idle',
      sessionError: null,
      sessionReplayTurn: patch.replayTurn,
      activeSkill: patch.activeSkill,
      lastCompaction: patch.lastCompaction,
    })
  }
}

export function createApplySessionHydrationAction(set: SetStoreState): ApplySessionHydration {
  return (input) => {
    set({
      session_id: input.sessionId,
      sessionRuntimeState: input.runtimeState,
      sessionHydrationStatus: input.hydrationStatus,
      sessionCanContinue: input.canContinue,
      sessionCanResume: input.canResume,
      sessionReadonlyReason: input.readonlyReason,
      sessionWarnings: [...input.warnings],
      sessionLastTurn: input.lastTurn,
      sessionNextTurnId: input.nextTurnId,
      sessionRevision: input.sessionRevision,
      sessionHydrationSource: input.hydrationSource,
      sessionError: null,
    })
  }
}

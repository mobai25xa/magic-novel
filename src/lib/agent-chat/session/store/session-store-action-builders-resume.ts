import { resumePersistedSession } from '../session-controller'
import { mergeSessionMetaToList } from './session-store-runtime'
import {
  createStateHelpers,
  logSessionMetric,
  unknownErrorToString,
} from './session-store-action-utils'
import type {
  ApplySessionEvents,
  ApplySessionHydration,
  GetStoreState,
  SetStoreState,
} from './session-store-types'

export function createResumeSessionAction(input: {
  set: SetStoreState
  get: GetStoreState
  applySessionEvents: ApplySessionEvents
  applySessionHydration: ApplySessionHydration
}) {
  const state = createStateHelpers(input.set, input.get)

  return async (params: { projectPath: string; sessionId: string }) => {
    state.setLoading(true)
    state.setError(null)

    try {
      const loaded = await resumePersistedSession({
        projectPath: params.projectPath,
        sessionId: params.sessionId,
      })

      input.applySessionEvents({
        sessionId: params.sessionId,
        events: loaded.events,
        meta: loaded.meta,
        replayedAt: Date.now(),
      })

      input.applySessionHydration({
        sessionId: params.sessionId,
        hydrationStatus: loaded.hydration.hydrationStatus,
        runtimeState: loaded.hydration.runtimeState,
        canContinue: loaded.hydration.canContinue,
        canResume: loaded.hydration.canResume,
        readonlyReason: loaded.hydration.readonlyReason,
        warnings: loaded.hydration.warnings,
        lastTurn: loaded.hydration.lastTurn,
        nextTurnId: loaded.hydration.nextTurnId,
        sessionRevision: loaded.hydration.sessionRevision,
        hydrationSource: loaded.hydration.hydrationSource,
      })

      logSessionMetric({
        get: input.get,
        metric: 'session_load_success_count',
        sessionId: params.sessionId,
        tags: {
          action: 'resume',
          project_path: params.projectPath,
        },
      })

      logSessionMetric({
        get: input.get,
        metric: 'session_hydrate_success_count',
        sessionId: params.sessionId,
        tags: {
          hydration_status: loaded.hydration.hydrationStatus,
          runtime_state: loaded.hydration.runtimeState,
          can_continue: loaded.hydration.canContinue,
          can_resume: loaded.hydration.canResume,
        },
      })

      if (!loaded.meta) {
        input.set({
          wasSessionResumed: true,
          pendingSessionReminder: {
            kind: 'resumed_session',
            payload: {
              resumed_from_session_id: params.sessionId,
              last_turn: loaded.hydration.lastTurn ?? null,
            next_turn_id: loaded.hydration.nextTurnId ?? null,
              last_stop_reason: null,
              last_compaction_at: null,
            hydration_source: loaded.hydration.hydrationSource ?? loaded.hydration.hydrationStatus,
            },
          },
        })
        return
      }

      const list = mergeSessionMetaToList({
        list: input.get().sessionList,
        meta: loaded.meta,
      })

      input.set({
        currentSessionMeta: loaded.meta,
        sessionList: list,
        wasSessionResumed: true,
        pendingSessionReminder: {
          kind: 'resumed_session',
          payload: {
            resumed_from_session_id: loaded.meta.session_id,
            last_turn: loaded.hydration.lastTurn ?? loaded.meta.last_turn ?? null,
            next_turn_id: loaded.hydration.nextTurnId ?? null,
            last_stop_reason: loaded.meta.last_stop_reason ?? null,
            last_compaction_at: null,
            hydration_source: loaded.hydration.hydrationSource ?? loaded.hydration.hydrationStatus,
          },
        },
        committedTimelineByTurnId: {},
      })
    } catch (error) {
      logSessionMetric({
        get: input.get,
        metric: 'session_load_error_count',
        sessionId: params.sessionId,
        tags: {
          action: 'resume',
          project_path: params.projectPath,
        },
      })

      logSessionMetric({
        get: input.get,
        metric: 'session_hydrate_error_count',
        sessionId: params.sessionId,
        tags: {
          action: 'resume',
        },
      })

      state.setError(unknownErrorToString(error))
      throw error
    } finally {
      state.setLoading(false)
    }
  }
}

import { createNewPersistedSession } from '../session-controller'
import {
  buildSessionRuntimeResetPatch,
  mergeSessionMetaToList,
} from './session-store-runtime'
import {
  createStateHelpers,
  logSessionMetric,
  unknownErrorToString,
} from './session-store-action-utils'
import type { ApplySessionEvents, GetStoreState, SetStoreState } from './session-store-types'

export function createEnsureSessionAction(input: {
  set: SetStoreState
  get: GetStoreState
}) {
  const state = createStateHelpers(input.set, input.get)

  return async (params: {
    projectPath: string
    title?: string
    activeChapterPath?: string
  }) => {
    if (state.getCurrentSessionId()) {
      return
    }

    state.setLoading(true)
    state.setError(null)

    try {
      const meta = await createNewPersistedSession({
        projectPath: params.projectPath,
        title: params.title,
        activeChapterPath: params.activeChapterPath,
      })

      input.set({
        ...buildSessionRuntimeResetPatch(meta),
        sessionList: mergeSessionMetaToList({
          list: input.get().sessionList,
          meta,
        }),
        wasSessionResumed: false,
        pendingSessionReminder: {
          kind: 'new_session',
          payload: {
            session_id: meta.session_id,
            active_chapter_path: meta.active_chapter_path ?? null,
            next_turn_id: 1,
            hydration_source: 'memory_hit',
          },
        },
        sessionRuntimeState: 'ready',
        sessionHydrationStatus: 'memory_hit',
        sessionCanContinue: true,
        sessionCanResume: false,
        sessionReadonlyReason: undefined,
        sessionWarnings: [],
        sessionReplayTurn: 0,
        sessionLastTurn: 0,
        sessionNextTurnId: 1,
        sessionRevision: undefined,
        sessionHydrationSource: 'memory_hit',
        activeSkill: undefined,
        lastCompaction: undefined,
        committedTimelineByTurnId: {},
      })

      logSessionMetric({
        get: input.get,
        metric: 'session_create_success_count',
        sessionId: meta.session_id,
        tags: {
          action: 'ensure',
          project_path: params.projectPath,
        },
      })
    } catch (error) {
      logSessionMetric({
        get: input.get,
        metric: 'session_create_error_count',
        tags: {
          action: 'ensure',
          project_path: params.projectPath,
        },
      })
      state.setError(unknownErrorToString(error))
      throw error
    } finally {
      state.setLoading(false)
    }
  }
}

export function createStartSessionAction(input: {
  set: SetStoreState
  get: GetStoreState
  applySessionEvents: ApplySessionEvents
}) {
  const state = createStateHelpers(input.set, input.get)

  return async (params: {
    projectPath: string
    title?: string
    activeChapterPath?: string
  }) => {
    state.setLoading(true)
    state.setError(null)

    try {
      const meta = await createNewPersistedSession({
        projectPath: params.projectPath,
        title: params.title,
        activeChapterPath: params.activeChapterPath,
      })

      input.set({
        ...buildSessionRuntimeResetPatch(meta),
        sessionList: mergeSessionMetaToList({
          list: input.get().sessionList,
          meta,
        }),
        wasSessionResumed: false,
        pendingSessionReminder: {
          kind: 'new_session',
          payload: {
            session_id: meta.session_id,
            active_chapter_path: meta.active_chapter_path ?? null,
            next_turn_id: 1,
            hydration_source: 'memory_hit',
          },
        },
        sessionRuntimeState: 'ready',
        sessionHydrationStatus: 'memory_hit',
        sessionCanContinue: true,
        sessionCanResume: false,
        sessionReadonlyReason: undefined,
        sessionWarnings: [],
        sessionReplayTurn: 0,
        sessionLastTurn: 0,
        sessionNextTurnId: 1,
        sessionRevision: undefined,
        sessionHydrationSource: 'memory_hit',
        activeSkill: undefined,
        lastCompaction: undefined,
        committedTimelineByTurnId: {},
      })

      input.applySessionEvents({
        sessionId: meta.session_id,
        events: [],
        meta,
      })

      logSessionMetric({
        get: input.get,
        metric: 'session_create_success_count',
        sessionId: meta.session_id,
        tags: {
          action: 'start_new',
          project_path: params.projectPath,
        },
      })
    } catch (error) {
      logSessionMetric({
        get: input.get,
        metric: 'session_create_error_count',
        tags: {
          action: 'start_new',
          project_path: params.projectPath,
        },
      })
      state.setError(unknownErrorToString(error))
      throw error
    } finally {
      state.setLoading(false)
    }
  }
}

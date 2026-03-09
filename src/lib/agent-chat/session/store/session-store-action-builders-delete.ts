import { deletePersistedSession, renamePersistedSession } from '../session-controller'
import { buildSessionRuntimeResetPatch, normalizeSessionList } from './session-store-runtime'
import {
  createStateHelpers,
  logSessionMetric,
  unknownErrorToString,
} from './session-store-action-utils'
import type { GetStoreState, SetStoreState } from './session-store-types'

export function createDeleteSessionAction(set: SetStoreState, get: GetStoreState) {
  const state = createStateHelpers(set, get)

  return async (input: { projectPath: string; sessionId: string }) => {
    state.setLoading(true)
    state.setError(null)

    try {
      const currentSessionId = get().currentSessionMeta?.session_id
      await deletePersistedSession({
        projectPath: input.projectPath,
        sessionId: input.sessionId,
      })

      const nextList = normalizeSessionList(
        get().sessionList.filter((item) => item.session_id !== input.sessionId),
      )

      if (currentSessionId === input.sessionId) {
        set({
          ...buildSessionRuntimeResetPatch(undefined),
          sessionList: nextList,
          wasSessionResumed: false,
          pendingSessionReminder: undefined,
          activeSkill: undefined,
          lastCompaction: undefined,
          committedTimelineByTurnId: {},
        })

        logSessionMetric({
          get,
          metric: 'session_delete_success_count',
          sessionId: input.sessionId,
          tags: {
            project_path: input.projectPath,
            deleted_active_session: true,
          },
        })
        return
      }

      set({ sessionList: nextList })

      logSessionMetric({
        get,
        metric: 'session_delete_success_count',
        sessionId: input.sessionId,
        tags: {
          project_path: input.projectPath,
          deleted_active_session: false,
        },
      })
    } catch (error) {
      logSessionMetric({
        get,
        metric: 'session_delete_error_count',
        sessionId: input.sessionId,
        tags: {
          project_path: input.projectPath,
        },
      })
      state.setError(unknownErrorToString(error))
      throw error
    } finally {
      state.setLoading(false)
    }
  }
}

export function createRenameSessionAction(set: SetStoreState, get: GetStoreState) {
  const state = createStateHelpers(set, get)

  return async (input: { projectPath: string; sessionId: string; title: string }) => {
    const nextTitle = input.title.trim()
    if (!nextTitle) {
      return
    }

    state.setLoading(true)
    state.setError(null)

    try {
      await renamePersistedSession({
        projectPath: input.projectPath,
        sessionId: input.sessionId,
        title: nextTitle,
      })

      const sessionList = normalizeSessionList(
        get().sessionList.map((item) => (
          item.session_id === input.sessionId
            ? {
              ...item,
              title: nextTitle,
              updated_at: Date.now(),
            }
            : item
        )),
      )

      const currentMeta = get().currentSessionMeta
      const currentSessionMeta = currentMeta?.session_id === input.sessionId
        ? {
          ...currentMeta,
          title: nextTitle,
          updated_at: Date.now(),
        }
        : currentMeta

      set({
        sessionList,
        currentSessionMeta,
      })

      logSessionMetric({
        get,
        metric: 'session_rename_success_count',
        sessionId: input.sessionId,
        tags: {
          project_path: input.projectPath,
        },
      })
    } catch (error) {
      logSessionMetric({
        get,
        metric: 'session_rename_error_count',
        sessionId: input.sessionId,
        tags: {
          project_path: input.projectPath,
        },
      })
      state.setError(unknownErrorToString(error))
      throw error
    } finally {
      state.setLoading(false)
    }
  }
}

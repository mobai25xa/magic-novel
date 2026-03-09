import { loadPersistedSessions } from '../session-controller'
import { normalizeSessionList } from './session-store-runtime'
import {
  createStateHelpers,
  logSessionMetric,
  unknownErrorToString,
} from './session-store-action-utils'
import type { GetStoreState, SetStoreState } from './session-store-types'

export function createLoadSessionListAction(set: SetStoreState, get: GetStoreState) {
  const state = createStateHelpers(set, get)

  return async (input: { projectPath: string; limit?: number }) => {
    state.setLoading(true)
    state.setError(null)

    try {
      const list = await loadPersistedSessions({
        projectPath: input.projectPath,
        limit: input.limit,
      })

      set({ sessionList: normalizeSessionList(list) })

      logSessionMetric({
        get,
        metric: 'session_list_load_success_count',
        tags: {
          project_path: input.projectPath,
          limit: typeof input.limit === 'number' ? input.limit : 'default',
        },
      })
    } catch (error) {
      logSessionMetric({
        get,
        metric: 'session_list_load_error_count',
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

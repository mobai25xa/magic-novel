import {
  deleteSessionAndReload,
  loadRecentSessions,
  renameSessionAndReload,
  resumeSessionAndReload,
  startNewSessionAndReload,
} from './agent-chat-panel-session-actions-store'
import { runSessionAction } from './agent-chat-panel-session-actions-runtime'

import type { AgentPanelError } from '../../agent-chat-panel-utils'

export function createSessionPanelActions(input: {
  setLastError: (value: AgentPanelError | null) => void
}) {
  return {
    startNewSession: () => runSessionAction(async (projectPath) => {
      await startNewSessionAndReload({ projectPath })
    }, { onError: input.setLastError }),

    loadSessionList: () => runSessionAction(
      (projectPath) => loadRecentSessions(projectPath),
      { onError: input.setLastError },
    ),

    resumeSession: (sessionId: string) => runSessionAction(async (projectPath) => {
      await resumeSessionAndReload({ projectPath, sessionId })
    }, { onError: input.setLastError }),

    renameSession: (sessionId: string, title: string) => runSessionAction(async (projectPath) => {
      await renameSessionAndReload({ projectPath, sessionId, title })
    }, { onError: input.setLastError }),

    deleteSession: (sessionId: string) => runSessionAction(async (projectPath) => {
      await deleteSessionAndReload({ projectPath, sessionId })
    }, { onError: input.setLastError }),
  }
}

import { useAgentChatStore } from '../store'

export function loadSessionList(input: {
  projectPath: string
  limit?: number
}) {
  return useAgentChatStore.getState().loadPersistedSessionList({
    projectPath: input.projectPath,
    limit: input.limit,
  })
}

export function startNewSession(input: {
  projectPath: string
  activeChapterPath?: string
}) {
  return useAgentChatStore.getState().startNewPersistedSession({
    projectPath: input.projectPath,
    activeChapterPath: input.activeChapterPath,
  })
}

export function resumeSession(input: {
  projectPath: string
  sessionId: string
}) {
  return useAgentChatStore.getState().resumePersistedSession({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })
}

export function deleteSession(input: {
  projectPath: string
  sessionId: string
}) {
  return useAgentChatStore.getState().deletePersistedSession({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })
}

export function renameSession(input: {
  projectPath: string
  sessionId: string
  title: string
}) {
  return useAgentChatStore.getState().renamePersistedSession({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    title: input.title,
  })
}

export function currentActiveChapterPath() {
  return useAgentChatStore.getState().active_chapter_path
}

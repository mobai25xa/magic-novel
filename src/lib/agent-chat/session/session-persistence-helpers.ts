import { useProjectStore } from '@/stores/project-store'

export function canPersistSession(sessionId?: string) {
  const projectPath = useProjectStore.getState().projectPath
  return Boolean(projectPath && sessionId)
}

export function currentProjectPath() {
  return useProjectStore.getState().projectPath
}

export function resolveSessionPersistenceInput(sessionId?: string) {
  if (!canPersistSession(sessionId)) {
    return null
  }

  const projectPath = currentProjectPath()
  if (!projectPath || !sessionId) {
    return null
  }

  return {
    projectPath,
    sessionId,
  }
}

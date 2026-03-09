import {
  currentActiveChapterPath,
  deleteSession,
  loadSessionList,
  renameSession,
  resumeSession,
  startNewSession,
} from '@/lib/agent-chat/session/session-persistence-store'
import type {
  AgentSessionHydrationStatus,
  AgentSessionReadonlyReason,
  AgentSessionRuntimeState,
} from '@/lib/agent-chat/session/session-types'
import { formatUnknownError } from '@/lib/error-utils'
import { useAgentChatStore } from '@/state/agent'

export type SessionRuntimeState = AgentSessionRuntimeState
export type SessionHydrationStatus = AgentSessionHydrationStatus

export interface SessionRuntimeCapability {
  runtimeState: SessionRuntimeState
  canContinue: boolean
  canResume: boolean
  readonlyReason?: AgentSessionReadonlyReason
  hydrationStatus?: SessionHydrationStatus
  warnings: string[]
}

type SessionRuntimeCapabilitySnapshot = {
  current: SessionRuntimeCapability
  bySessionId: Record<string, SessionRuntimeCapability>
  version: number
}


function createInteractiveCapability(): SessionRuntimeCapability {
  return {
    runtimeState: 'ready',
    canContinue: true,
    canResume: false,
    readonlyReason: undefined,
    hydrationStatus: undefined,
    warnings: [],
  }
}

let sessionRuntimeCapabilitySnapshot: SessionRuntimeCapabilitySnapshot = {
  current: createInteractiveCapability(),
  bySessionId: {},
  version: 0,
}

const sessionRuntimeCapabilityListeners = new Set<() => void>()

function emitSessionRuntimeCapabilitySnapshot(next: Omit<SessionRuntimeCapabilitySnapshot, 'version'>) {
  sessionRuntimeCapabilitySnapshot = {
    ...next,
    version: sessionRuntimeCapabilitySnapshot.version + 1,
  }

  sessionRuntimeCapabilityListeners.forEach((listener) => listener())
}

function setSessionRuntimeCapability(input: {
  sessionId: string
  capability: SessionRuntimeCapability
}) {
  emitSessionRuntimeCapabilitySnapshot({
    current: input.capability,
    bySessionId: {
      ...sessionRuntimeCapabilitySnapshot.bySessionId,
      [input.sessionId]: input.capability,
    },
  })
}

export function syncSessionRuntimeCapability(input: {
  sessionId: string
  capability: SessionRuntimeCapability
}) {
  setSessionRuntimeCapability(input)
}

function removeSessionRuntimeCapability(sessionId: string) {
  if (!sessionRuntimeCapabilitySnapshot.bySessionId[sessionId]) {
    return
  }

  const nextBySessionId = { ...sessionRuntimeCapabilitySnapshot.bySessionId }
  delete nextBySessionId[sessionId]

  emitSessionRuntimeCapabilitySnapshot({
    current: sessionRuntimeCapabilitySnapshot.current,
    bySessionId: nextBySessionId,
  })
}


export function subscribeSessionRuntimeCapability(listener: () => void) {
  sessionRuntimeCapabilityListeners.add(listener)
  return () => {
    sessionRuntimeCapabilityListeners.delete(listener)
  }
}

export function getSessionRuntimeCapabilitySnapshot() {
  return sessionRuntimeCapabilitySnapshot
}

export function resetSessionRuntimeCapabilitySnapshot() {
  emitSessionRuntimeCapabilitySnapshot({
    current: createInteractiveCapability(),
    bySessionId: {},
  })
}

export async function startNewSessionAndReload(input: {
  projectPath: string
}) {
  await startNewSession({
    projectPath: input.projectPath,
    activeChapterPath: currentActiveChapterPath(),
  })

  const sessionId = useAgentChatStore.getState().session_id
  if (sessionId) {
    setSessionRuntimeCapability({
      sessionId,
      capability: createInteractiveCapability(),
    })
  }

  return loadSessionList({ projectPath: input.projectPath, limit: 20 })
}

export async function resumeSessionAndReload(input: {
  projectPath: string
  sessionId: string
}) {
  try {
    await resumeSession({
      projectPath: input.projectPath,
      sessionId: input.sessionId,
    })
  } catch (error) {
    useAgentChatStore.setState({
      sessionError: formatUnknownError(error, 'E_AGENT_SESSION_RESUME_FAILED'),
    })
    throw error
  }

  const state = useAgentChatStore.getState()
  setSessionRuntimeCapability({
    sessionId: input.sessionId,
    capability: {
      runtimeState: state.sessionRuntimeState ?? 'degraded',
      canContinue: state.sessionCanContinue,
      canResume: state.sessionCanResume,
      readonlyReason: state.sessionReadonlyReason,
      hydrationStatus: state.sessionHydrationStatus,
      warnings: state.sessionWarnings,
    },
  })

  return loadSessionList({ projectPath: input.projectPath, limit: 20 })
}

export function loadRecentSessions(projectPath: string) {
  return loadSessionList({ projectPath, limit: 20 })
}

export async function deleteSessionAndReload(input: {
  projectPath: string
  sessionId: string
}) {
  const previousCurrentSessionId = useAgentChatStore.getState().currentSessionMeta?.session_id

  await deleteSession({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })

  removeSessionRuntimeCapability(input.sessionId)

  const nextSessionId = useAgentChatStore.getState().session_id
  if (previousCurrentSessionId === input.sessionId && nextSessionId) {
    setSessionRuntimeCapability({
      sessionId: nextSessionId,
      capability: createInteractiveCapability(),
    })
  }

  return loadSessionList({ projectPath: input.projectPath, limit: 20 })
}

export async function renameSessionAndReload(input: {
  projectPath: string
  sessionId: string
  title: string
}) {
  await renameSession({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    title: input.title,
  })

  return loadSessionList({ projectPath: input.projectPath, limit: 20 })
}

export function ensureSessionRuntimeCapability(input: {
  sessionId: string
  capability?: SessionRuntimeCapability
}) {
  const existing = sessionRuntimeCapabilitySnapshot.bySessionId[input.sessionId]
  if (existing) {
    emitSessionRuntimeCapabilitySnapshot({
      current: existing,
      bySessionId: sessionRuntimeCapabilitySnapshot.bySessionId,
    })
    return
  }

  setSessionRuntimeCapability({
    sessionId: input.sessionId,
    capability: input.capability ?? createInteractiveCapability(),
  })
}

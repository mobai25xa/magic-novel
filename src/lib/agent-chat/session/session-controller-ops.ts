import {
  AGENT_SESSION_SCHEMA_VERSION,
  type AgentSessionEvent,
  type AgentSessionMeta,
} from './session-types'
import {
  inferHistoricalLastTurn,
  normalizeSessionHydration,
} from './session-hydration'
import {
  createPersistedSessionClient,
  deletePersistedSessionClient,
  hydratePersistedSessionClient,
  listPersistedSessionsClient,
  loadPersistedSessionClient,
  renamePersistedSessionClient,
  type CreatePersistedSessionInput,
  type HydratePersistedSessionOutput,
} from './session-client'
import {
  buildEphemeralMeta,
  clearSessionControllerState,
  getActiveSession,
  setActiveSession,
} from './session-controller-state'

function normalizeList(items: AgentSessionMeta[]) {
  return [...items].sort((a, b) => b.updated_at - a.updated_at)
}

export async function ensurePersistedSession(input: CreatePersistedSessionInput): Promise<AgentSessionMeta> {
  const meta = buildEphemeralMeta(input)
  if (meta) {
    return meta
  }

  const created = await createPersistedSessionClient(input)
  setActiveSession({
    projectPath: input.projectPath,
    sessionId: created.session_id,
  })
  return created
}

export async function createNewPersistedSession(input: CreatePersistedSessionInput): Promise<AgentSessionMeta> {
  const active = getActiveSession()
  if (active?.projectPath === input.projectPath) {
    clearSessionControllerState()
  }

  const created = await createPersistedSessionClient(input)
  setActiveSession({
    projectPath: input.projectPath,
    sessionId: created.session_id,
  })
  return created
}

export async function loadPersistedSessions(input: { projectPath: string; limit?: number }): Promise<AgentSessionMeta[]> {
  const items = await listPersistedSessionsClient({
    projectPath: input.projectPath,
    limit: input.limit,
  })

  return normalizeList(items)
}

export async function deletePersistedSession(input: {
  projectPath: string
  sessionId: string
}) {
  const active = getActiveSession()
  if (active?.projectPath === input.projectPath && active.sessionId === input.sessionId) {
    clearSessionControllerState()
  }

  await deletePersistedSessionClient({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })
}

export async function renamePersistedSession(input: {
  projectPath: string
  sessionId: string
  title: string
}) {
  const nextTitle = input.title.trim()
  if (!nextTitle) {
    return
  }

  await renamePersistedSessionClient({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    title: nextTitle,
  })
}

type ResumePersistedSessionResult = {
  meta?: AgentSessionMeta
  events: AgentSessionEvent[]
  hydration: HydratePersistedSessionOutput
}

async function loadSessionForResume(input: {
  projectPath: string
  sessionId: string
}): Promise<{ meta?: AgentSessionMeta; events: AgentSessionEvent[] }> {
  const loaded = await loadPersistedSessionClient({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })

  return {
    meta: loaded.meta,
    events: loaded.events,
  }
}

function fallbackHydrationFromHistory(input: {
  sessionId: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
}): HydratePersistedSessionOutput {
  const sorted = [...input.events].sort((left, right) => {
    const leftSeq = typeof left.event_seq === 'number' ? left.event_seq : Number.MAX_SAFE_INTEGER
    const rightSeq = typeof right.event_seq === 'number' ? right.event_seq : Number.MAX_SAFE_INTEGER

    if (leftSeq !== rightSeq) {
      return leftSeq - rightSeq
    }

    if (left.ts !== right.ts) {
      return left.ts - right.ts
    }

    const leftId = typeof left.event_id === 'string' ? left.event_id : ''
    const rightId = typeof right.event_id === 'string' ? right.event_id : ''
    return leftId.localeCompare(rightId)
  })

  const lastState = [...sorted]
    .reverse()
    .find((event) => event.type === 'turn_state')
    ?.payload?.state

  const normalizedState = typeof lastState === 'string' ? lastState : undefined
  const isSuspended = normalizedState === 'waiting_confirmation' || normalizedState === 'waiting_askuser'

  const runtimeState = isSuspended ? 'degraded' : 'ready'
  const readonlyReason = isSuspended
    ? 'historical_suspended_session_without_runtime_snapshot'
    : undefined
  const lastTurn = inferHistoricalLastTurn({
    events: input.events,
    meta: input.meta,
  })

  return normalizeSessionHydration({
    schemaVersion: AGENT_SESSION_SCHEMA_VERSION,
    sessionId: input.sessionId,
    hydrationStatus: isSuspended ? 'readonly_fallback' : 'event_rebuilt',
    runtimeState,
    canContinue: !isSuspended,
    canResume: false,
    readonlyReason,
    warnings: isSuspended
      ? ['snapshot_missing_for_suspended_session', 'resume_requires_runtime_snapshot']
      : ['runtime_snapshot_rebuilt_from_event_log'],
    lastTurn,
  })
}

async function hydrateSessionForResume(input: {
  projectPath: string
  sessionId: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
}): Promise<HydratePersistedSessionOutput> {
  try {
    return await hydratePersistedSessionClient({
      projectPath: input.projectPath,
      sessionId: input.sessionId,
    })
  } catch {
    return fallbackHydrationFromHistory({
      sessionId: input.sessionId,
      events: input.events,
      meta: input.meta,
    })
  }
}

export async function resumePersistedSession(input: {
  projectPath: string
  sessionId: string
}): Promise<ResumePersistedSessionResult> {
  const loaded = await loadSessionForResume(input)

  setActiveSession({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })

  const hydration = await hydrateSessionForResume({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    events: loaded.events,
    meta: loaded.meta,
  })

  return {
    meta: loaded.meta,
    events: loaded.events,
    hydration,
  }
}

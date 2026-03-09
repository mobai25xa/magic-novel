import { createEmptyTodoState } from '@/agent/types'
import type { TurnTimelineSnapshot } from '../../timeline'

import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnState,
} from '../../types'
import type {
  SessionPersistenceState,
  SessionPersistenceStorePatch,
  SessionRuntimeStoreState,
} from './session-store-contract'

function createSessionId() {
  return `chat_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
}

function createRuntimeDefaults(): SessionRuntimeStoreState {
  return {
    session_id: createSessionId(),
    turn: 0,
    active_chapter_path: undefined,
    activeSkill: undefined,
    messages: [],
    traces: [],
    llmMessages: [],
    stateStatus: 'idle',
    lastStopReason: undefined,
    lastTurnLatencyMs: undefined,
    turnOrder: [],
    turnById: {} as Record<number, AgentUiTurnState>,
    answerByTurnId: {} as Record<number, string>,
    thinkingByTurnId: {} as Record<number, string>,
    stepsByTurnId: {} as Record<number, AgentUiToolStep[]>,
    eventsByTurnId: {} as Record<number, AgentUiTimelineEvent[]>,
    committedTimelineByTurnId: {} as Record<number, TurnTimelineSnapshot>,
    pendingAskUser: undefined,
  }
}

function createPersistenceDefaults(): SessionPersistenceState {
  return {
    currentSessionMeta: undefined,
    sessionList: [],
    wasSessionResumed: false,
    pendingSessionReminder: undefined,
    lastCompaction: undefined,
    isSessionLoading: false,
    sessionError: null,
    sessionRuntimeState: undefined,
    sessionHydrationStatus: undefined,
    sessionCanContinue: false,
    sessionCanResume: false,
    sessionReadonlyReason: undefined,
    sessionWarnings: [],
    sessionReplayTurn: 0,
    sessionLastTurn: undefined,
    sessionNextTurnId: undefined,
    sessionRevision: undefined,
    sessionHydrationSource: undefined,
    todoState: createEmptyTodoState(),
  }
}

export function createInitialSessionStorePatch(input?: {
  sessionId?: string
}): SessionPersistenceStorePatch {
  const runtime = createRuntimeDefaults()
  const persistence = createPersistenceDefaults()

  return {
    ...runtime,
    ...persistence,
    ...(input?.sessionId ? { session_id: input.sessionId } : {}),
  }
}

function uniqueBySessionId(input: import('../session-types').AgentSessionMeta[]) {
  const map = new Map<string, import('../session-types').AgentSessionMeta>()
  for (const item of input) {
    map.set(item.session_id, item)
  }
  return [...map.values()].sort((a, b) => b.updated_at - a.updated_at)
}

export function mergeSessionMetaToList(input: {
  list: import('../session-types').AgentSessionMeta[]
  meta: import('../session-types').AgentSessionMeta
}) {
  return uniqueBySessionId([input.meta, ...input.list])
}

export function normalizeSessionList(list: import('../session-types').AgentSessionMeta[]) {
  return uniqueBySessionId(list)
}

export function buildSessionRuntimeResetPatch(meta?: import('../session-types').AgentSessionMeta): SessionPersistenceStorePatch {
  const base = createInitialSessionStorePatch({
    sessionId: meta?.session_id,
  })

  if (!meta) {
    return base
  }

  return {
    ...base,
    active_chapter_path: meta.active_chapter_path,
    currentSessionMeta: meta,
  }
}

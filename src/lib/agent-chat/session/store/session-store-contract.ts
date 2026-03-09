import type {
  AgentPendingAskUserRequest,
  AgentCompactionMeta,
  AgentLoopStopReason,
  AgentSessionReminder,
  AgentStateStatus,
  AgentTodoState,
} from '@/agent/types'

import type { TurnTimelineSnapshot } from '../../timeline'

import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnState,
  ChatToolTrace,
  ChatUiMessage,
  OpenAiMessage,
} from '../../types'
import type {
  AgentSessionHydrationAuthority,
  AgentSessionHydrationStatus,
  AgentSessionMeta,
  AgentSessionReadonlyReason,
  AgentSessionRuntimeState,
} from '../session-types'

export interface SessionPersistenceState {
  currentSessionMeta?: AgentSessionMeta
  sessionList: AgentSessionMeta[]
  wasSessionResumed: boolean
  pendingSessionReminder?: AgentSessionReminder
  lastCompaction?: AgentCompactionMeta
  isSessionLoading: boolean
  sessionError: string | null
  sessionRuntimeState?: AgentSessionRuntimeState
  sessionHydrationStatus?: AgentSessionHydrationStatus
  sessionCanContinue: boolean
  sessionCanResume: boolean
  sessionReadonlyReason?: AgentSessionReadonlyReason
  sessionWarnings: string[]
  sessionReplayTurn: number
  sessionLastTurn?: number
  sessionNextTurnId?: number
  sessionRevision?: number
  sessionHydrationSource?: string
  todoState: AgentTodoState
}

export interface SessionRuntimeStoreState {
  session_id: string
  turn: number
  active_chapter_path?: string
  activeSkill?: string
  messages: ChatUiMessage[]
  traces: ChatToolTrace[]
  llmMessages: OpenAiMessage[]
  stateStatus: AgentStateStatus
  lastStopReason?: AgentLoopStopReason
  lastTurnLatencyMs?: number
  turnOrder: number[]
  turnById: Record<number, AgentUiTurnState>
  answerByTurnId: Record<number, string>
  thinkingByTurnId: Record<number, string>
  stepsByTurnId: Record<number, AgentUiToolStep[]>
  eventsByTurnId: Record<number, AgentUiTimelineEvent[]>
  committedTimelineByTurnId: Record<number, TurnTimelineSnapshot>
  pendingAskUser?: AgentPendingAskUserRequest
}

export interface SessionPersistenceStorePatch {
  currentSessionMeta?: AgentSessionMeta
  sessionList: AgentSessionMeta[]
  wasSessionResumed: boolean
  pendingSessionReminder?: AgentSessionReminder
  lastCompaction?: AgentCompactionMeta
  isSessionLoading: boolean
  sessionError: string | null
  sessionRuntimeState?: AgentSessionRuntimeState
  sessionHydrationStatus?: AgentSessionHydrationStatus
  sessionCanContinue: boolean
  sessionCanResume: boolean
  sessionReadonlyReason?: AgentSessionReadonlyReason
  sessionWarnings: string[]
  sessionReplayTurn: number
  sessionLastTurn?: number
  sessionNextTurnId?: number
  sessionRevision?: number
  sessionHydrationSource?: string
  todoState: AgentTodoState
  session_id: string
  turn: number
  active_chapter_path?: string
  activeSkill?: string
  messages: ChatUiMessage[]
  traces: ChatToolTrace[]
  llmMessages: OpenAiMessage[]
  stateStatus: AgentStateStatus
  lastStopReason?: AgentLoopStopReason
  lastTurnLatencyMs?: number
  turnOrder: number[]
  turnById: Record<number, AgentUiTurnState>
  answerByTurnId: Record<number, string>
  thinkingByTurnId: Record<number, string>
  stepsByTurnId: Record<number, AgentUiToolStep[]>
  eventsByTurnId: Record<number, AgentUiTimelineEvent[]>
  committedTimelineByTurnId: Record<number, TurnTimelineSnapshot>
  pendingAskUser?: AgentPendingAskUserRequest
}

export type SessionPersistenceStoreActions = {
  ensurePersistedSession: (input: {
    projectPath: string
    title?: string
    activeChapterPath?: string
  }) => Promise<void>
  startNewPersistedSession: (input: {
    projectPath: string
    title?: string
    activeChapterPath?: string
  }) => Promise<void>
  loadPersistedSessionList: (input: { projectPath: string; limit?: number }) => Promise<void>
  resumePersistedSession: (input: { projectPath: string; sessionId: string }) => Promise<void>
  renamePersistedSession: (input: { projectPath: string; sessionId: string; title: string }) => Promise<void>
  deletePersistedSession: (input: { projectPath: string; sessionId: string }) => Promise<void>
  consumeWasSessionResumed: () => boolean
  consumeSessionReminder: () => AgentSessionReminder | undefined
  applySessionEvents: (input: {
    sessionId: string
    events: import('../session-types').AgentSessionEvent[]
    meta?: AgentSessionMeta
    replayedAt?: number
  }) => void
  applySessionHydration: (input: {
    sessionId: string
    hydrationStatus: AgentSessionHydrationStatus
    runtimeState: AgentSessionRuntimeState
    canContinue: boolean
    canResume: boolean
    readonlyReason?: AgentSessionReadonlyReason
    warnings: string[]
  } & AgentSessionHydrationAuthority) => void
}

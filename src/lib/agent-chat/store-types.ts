import type {
  AgentAskUserAnswer,
  AgentPendingAskUserRequest,
  AgentCompactionMeta,
  AgentLoopStopReason,
  AgentSessionReminder,
  AgentStateStatus,
  AgentTodoState,
} from '@/agent/types'

import type { TurnTimelineSnapshot } from './timeline'

import type {
  AgentSessionEvent,
  AgentSessionHydrationAuthority,
  AgentSessionHydrationStatus,
  AgentSessionMeta,
  AgentSessionReadonlyReason,
  AgentSessionRuntimeState,
} from './session'
import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnError,
  AgentUiTurnPhase,
  AgentUiTurnState,
  AgentUiTurnView,
  ChatToolTrace,
  ChatUiMessage,
  FeedbackRating,
  OpenAiMessage,
  RunChatTurnOptions,
} from './types'
import type {
  ToolStepCompleteInput,
  ToolStepProgressInput,
  ToolStepStartInput,
  ToolStepWaitingInput,
} from './store-helpers-tool-steps'

export interface AgentPendingTurnRequest {
  clientRequestId: string
  sessionId: string
  createdAt: number
  status: 'starting' | 'cancel_requested'
}

export interface BindPendingTurnRequestResult {
  alreadyBound: boolean
  turn: number
  cancelRequested: boolean
  messagesToPersist: ChatUiMessage[]
}

export interface AgentChatState {
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

  turnFeedback: Record<number, FeedbackRating>

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
  pendingAskUser?: AgentPendingAskUserRequest
  pendingRequestsByClientRequestId: Record<string, AgentPendingTurnRequest>
  boundTurnByClientRequestId: Record<string, number>
  pendingUserMessageIdsByClientRequestId: Record<string, string[]>
  clientRequestIdByTurnId: Record<number, string>

  nextTurn: () => number
  setTurn: (turn: number) => void
  setActiveChapterPath: (path?: string) => void
  setActiveSkill: (skill?: string) => void
  addUiMessage: (message: ChatUiMessage) => void
  addTrace: (trace: ChatToolTrace) => void
  pushLlmMessage: (message: OpenAiMessage) => void
  setLlmMessages: (messages: OpenAiMessage[]) => void
  setStateStatus: (status: AgentStateStatus) => void
  setSessionRuntimeCapability: (input: {
    runtimeState: AgentSessionRuntimeState
    canContinue: boolean
    canResume: boolean
    readonlyReason?: AgentSessionReadonlyReason
    hydrationStatus?: AgentSessionHydrationStatus
    warnings?: string[]
  }) => void
  setLastStopReason: (reason?: AgentLoopStopReason) => void
  setLastTurnLatency: (latencyMs?: number) => void
  markTurnStarted: (turn: number) => void
  startPendingTurnRequest: (input: { clientRequestId: string; userMessage: ChatUiMessage }) => void
  requestPendingTurnCancellation: (clientRequestId: string) => boolean
  bindPendingTurnRequest: (input: {
    clientRequestId: string
    turn: number
  }) => BindPendingTurnRequestResult
  failPendingTurnRequest: (input: {
    clientRequestId: string
    removePendingMessages?: boolean
  }) => void
  clearPendingTurnState: () => void
  setTurnPhase: (
    turn: number,
    phase: AgentUiTurnPhase,
    options?: { stopReason?: AgentLoopStopReason; error?: string; turnError?: AgentUiTurnError; finishedAt?: number },
  ) => void
  appendTurnAnswerDelta: (turn: number, delta: string) => void
  appendTurnThinkingDelta: (turn: number, delta: string) => void
  markToolStepStarted: (turn: number, input: ToolStepStartInput) => void
  markToolStepProgress: (turn: number, input: ToolStepProgressInput) => void
  markToolStepCompleted: (turn: number, input: ToolStepCompleteInput) => void
  markWaitingForConfirmation: (turn: number, input: ToolStepWaitingInput) => void
  pushTurnEvent: (
    turn: number,
    event: Omit<AgentUiTimelineEvent, 'id' | 'turn' | 'ts' | 'seq'> & { id?: string; ts?: number; seq?: number },
  ) => void
  commitTurnTimelineSnapshot: (turn: number) => void
  setTurnFeedback: (turnId: number, rating: FeedbackRating) => void
  getTurnViews: () => AgentUiTurnView[]
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
  resetForProjectSwitch: () => void
  applySessionEvents: (input: {
    sessionId: string
    events: AgentSessionEvent[]
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
  applyTodoState: (state: AgentTodoState) => void
  openAskUserRequest: (request: AgentPendingAskUserRequest) => void
  resolveAskUserRequest: (callId: string, answers: AgentAskUserAnswer[]) => void
  cancelAskUserRequest: (callId: string) => void
  clearPendingAskUser: () => void
  reset: () => void
  runChatTurn: (inputText: string, options?: RunChatTurnOptions) => Promise<string>
  retryTurn: (turn: AgentUiTurnState, options?: RunChatTurnOptions) => Promise<string>
  retryStep: (turnId: number, callId: string, options?: RunChatTurnOptions) => Promise<string>
}

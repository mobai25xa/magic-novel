import { create } from 'zustand'

import type { AgentPendingAskUserRequest } from '@/agent/types'
import { buildTurnTimelineSnapshot } from './timeline'
import { logUiMetric } from '@/agent/telemetry'

import {
  clearPersistedSessionController,
  createInitialSessionStorePatch,
  createSessionPersistenceActions,
} from './session'
import type { AgentChatState } from './store-types'
import { runChatTurn } from './runtime'

import {
  reduceAddUiMessage,
  reduceAppendTurnAnswerDelta,
  reduceAppendTurnThinkingDelta,
  reduceMarkTurnStarted,
  reduceSetStateStatus,
  reduceSetTurnPhase,
  pushWithLimit,
  withLimit,
} from './store-helpers'
import { reducePushTurnEvent } from './store-helpers-tools'
import {
  reduceMarkWaitingForConfirmation,
  reduceMarkAskUserStepAnswered,
  reduceUpsertToolStepCompleted,
  reduceUpsertToolStepProgress,
  reduceUpsertToolStepStarted,
} from './store-helpers-tool-steps'
import { selectTurnViews } from './store-selectors'
import { agentTurnResumeClient } from '@/platform/tauri/clients/agent-engine-client'
import type { FeedbackRating, RunChatTurnOptions } from './types'

const MAX_TOOL_TRACES = 500
const MAX_LLM_MESSAGES = 160

function asRecord(input: unknown): Record<string, unknown> | null {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return null
  }

  return input as Record<string, unknown>
}

function asText(input: unknown): string | undefined {
  if (typeof input !== 'string') {
    return undefined
  }

  const value = input.trim()
  return value || undefined
}

function extractErrorCode(error: unknown): string | undefined {
  const record = asRecord(error)
  const details = asRecord(record?.details)
  return asText(details?.code) || asText(record?.code)
}

function isResumeRuntimeUnavailableError(error: unknown) {
  const code = extractErrorCode(error)
  if (code === 'E_AGENT_SESSION_RUNTIME_UNAVAILABLE' || code === 'E_AGENT_SESSION_RESUME_NOT_SUPPORTED') {
    return true
  }

  const message = asText(asRecord(error)?.message)?.toLowerCase()
  if (!message) {
    return false
  }

  return message.includes('no suspended turn for session')
    || message.includes('must be resumed, not started')
}

function initialState() {
  return {
    ...createInitialSessionStorePatch(),
    turnFeedback: {} as Record<number, FeedbackRating>,
    pendingAskUser: undefined as AgentPendingAskUserRequest | undefined,
    pendingRequestsByClientRequestId: {} as AgentChatState['pendingRequestsByClientRequestId'],
    boundTurnByClientRequestId: {} as AgentChatState['boundTurnByClientRequestId'],
    pendingUserMessageIdsByClientRequestId: {} as AgentChatState['pendingUserMessageIdsByClientRequestId'],
    clientRequestIdByTurnId: {} as AgentChatState['clientRequestIdByTurnId'],
  }
}

function createTurnBindingConflictError(input: {
  clientRequestId: string
  existingTurn: number
  nextTurn: number
}) {
  const error = new Error(
    `E_AGENT_TURN_BINDING_CONFLICT:${input.clientRequestId}:${input.existingTurn}:${input.nextTurn}`,
  ) as Error & {
    code?: string
    detail?: typeof input
  }

  error.name = 'E_AGENT_TURN_BINDING_CONFLICT'
  error.code = 'E_AGENT_TURN_BINDING_CONFLICT'
  error.detail = input
  return error
}

function resolvePendingRequestKeyForTurnBinding(state: AgentChatState, clientRequestId: string): string | undefined {
  if (
    state.pendingRequestsByClientRequestId[clientRequestId]
    || state.pendingUserMessageIdsByClientRequestId[clientRequestId]
  ) {
    return clientRequestId
  }

  const pendingRequests = Object.values(state.pendingRequestsByClientRequestId)
    .filter((request) => request.sessionId === state.session_id)
    .filter((request) => typeof state.boundTurnByClientRequestId[request.clientRequestId] !== 'number')

  if (pendingRequests.length === 0) {
    const pendingMessageKeys = Object.keys(state.pendingUserMessageIdsByClientRequestId)
    if (pendingMessageKeys.length === 1) {
      return pendingMessageKeys[0]
    }
    return undefined
  }

  return [...pendingRequests]
    .sort((left, right) => left.createdAt - right.createdAt)
    .at(0)?.clientRequestId
}

function clearPendingTurnStatePatch(state: AgentChatState) {
  const pendingMessageIds = new Set(
    Object.values(state.pendingUserMessageIdsByClientRequestId).flat(),
  )

  return {
    messages: pendingMessageIds.size > 0
      ? state.messages.filter((message) => !pendingMessageIds.has(message.id))
      : state.messages,
    pendingRequestsByClientRequestId: {},
    boundTurnByClientRequestId: {},
    pendingUserMessageIdsByClientRequestId: {},
    clientRequestIdByTurnId: {},
  } satisfies Partial<AgentChatState>
}

function commitTurnTimelineSnapshotLocal(state: AgentChatState, turn: number) {
  const turnState = state.turnById[turn]
  if (!turnState) {
    return null
  }

  const snapshot = buildTurnTimelineSnapshot({
    turn,
    events: state.eventsByTurnId[turn] || [],
    toolStepsByCallId: Object.fromEntries((state.stepsByTurnId[turn] || []).map((step) => [step.callId, step])),
    answerText: state.answerByTurnId[turn] || '',
    thinkingText: state.thinkingByTurnId[turn] || '',
    running: false,
    phase: turnState.phase,
  })

  return {
    ...state.committedTimelineByTurnId,
    [turn]: snapshot,
  }
}

export const useAgentChatStore = create<AgentChatState>((set, get) => {
  const sessionActions = createSessionPersistenceActions({
    set: (next) => set(next),
    get: () => get(),
  })

  const clearPendingTurnStateLocal = () => {
    set((state) => clearPendingTurnStatePatch(state))
  }

  return {
    ...initialState(),
    ...sessionActions,

    ensurePersistedSession: async (input) => sessionActions.ensurePersistedSession(input),

    startNewPersistedSession: async (input) => {
      clearPendingTurnStateLocal()
      return sessionActions.startNewPersistedSession(input)
    },

    loadPersistedSessionList: sessionActions.loadPersistedSessionList,

    resumePersistedSession: async (input) => {
      clearPendingTurnStateLocal()
      return sessionActions.resumePersistedSession(input)
    },

    renamePersistedSession: sessionActions.renamePersistedSession,

    deletePersistedSession: async (input) => {
      clearPendingTurnStateLocal()
      return sessionActions.deletePersistedSession(input)
    },

    consumeWasSessionResumed: sessionActions.consumeWasSessionResumed,

    applySessionEvents: (input) => {
      clearPendingTurnStateLocal()
      sessionActions.applySessionEvents(input)
    },

    applySessionHydration: sessionActions.applySessionHydration,

    nextTurn: () => {
      const next = get().turn + 1
      set({ turn: next })
      return next
    },

    setTurn: (turn) => set({ turn }),

    setActiveChapterPath: (path) => set({ active_chapter_path: path }),

    setActiveSkill: (skill) => set({ activeSkill: skill?.trim() || undefined }),

    addUiMessage: (message) =>
      set((state) => reduceAddUiMessage(state, message)),

    addTrace: (trace) =>
      set((state) => ({ traces: pushWithLimit(state.traces, trace, MAX_TOOL_TRACES) })),

    pushLlmMessage: (message) =>
      set((state) => ({ llmMessages: pushWithLimit(state.llmMessages, message, MAX_LLM_MESSAGES) })),

    setLlmMessages: (messages) => set({ llmMessages: withLimit(messages, MAX_LLM_MESSAGES) }),

    setStateStatus: (status) =>
      set((state) => reduceSetStateStatus(state, status)),

    setSessionRuntimeCapability: (input) =>
      set((state) => ({
        sessionRuntimeState: input.runtimeState,
        sessionCanContinue: input.canContinue,
        sessionCanResume: input.canResume,
        sessionReadonlyReason: input.readonlyReason,
        sessionHydrationStatus: input.hydrationStatus ?? state.sessionHydrationStatus,
        sessionWarnings: input.warnings ? [...input.warnings] : state.sessionWarnings,
      })),

    setLastStopReason: (reason) => set({ lastStopReason: reason }),

    setLastTurnLatency: (latencyMs) => set({ lastTurnLatencyMs: latencyMs }),

    consumeSessionReminder: () => {
      const reminder = get().pendingSessionReminder
      if (reminder) {
        set({ pendingSessionReminder: undefined })
      }
      return reminder
    },

    resetForProjectSwitch: () => {
      clearPersistedSessionController()
      set(initialState())
    },

    markTurnStarted: (turn) =>
      set((state) => ({
        turn,
        ...reduceMarkTurnStarted(state, turn),
      })),

    startPendingTurnRequest: ({ clientRequestId, userMessage }) =>
      set((state) => ({
        turn: 0,
        messages: pushWithLimit(state.messages, userMessage, 300),
        pendingRequestsByClientRequestId: {
          ...state.pendingRequestsByClientRequestId,
          [clientRequestId]: {
            clientRequestId,
            sessionId: state.session_id,
            createdAt: userMessage.ts,
            status: 'starting',
          },
        },
        pendingUserMessageIdsByClientRequestId: {
          ...state.pendingUserMessageIdsByClientRequestId,
          [clientRequestId]: [
            ...(state.pendingUserMessageIdsByClientRequestId[clientRequestId] || []),
            userMessage.id,
          ],
        },
      })),

    requestPendingTurnCancellation: (clientRequestId) => {
      const pendingRequest = get().pendingRequestsByClientRequestId[clientRequestId]
      if (!pendingRequest) {
        return false
      }

      if (pendingRequest.status === 'cancel_requested') {
        return true
      }

      const pendingMessageIds = get().pendingUserMessageIdsByClientRequestId[clientRequestId] || []
      const pendingMessageIdSet = new Set(pendingMessageIds)

      set((state) => ({
        stateStatus: 'idle',
        messages: pendingMessageIdSet.size > 0
          ? state.messages.filter((message) => !pendingMessageIdSet.has(message.id))
          : state.messages,
        pendingRequestsByClientRequestId: {
          ...state.pendingRequestsByClientRequestId,
          [clientRequestId]: {
            ...state.pendingRequestsByClientRequestId[clientRequestId],
            status: 'cancel_requested',
          },
        },
      }))

      return true
    },

    bindPendingTurnRequest: ({ clientRequestId, turn }) => {
      const state = get()
      const existingTurn = state.boundTurnByClientRequestId[clientRequestId]
      if (typeof existingTurn === 'number') {
        if (existingTurn !== turn) {
          throw createTurnBindingConflictError({
            clientRequestId,
            existingTurn,
            nextTurn: turn,
          })
        }

        return {
          alreadyBound: true,
          turn,
          cancelRequested: false,
          messagesToPersist: [],
        }
      }

      const existingClientRequestId = state.clientRequestIdByTurnId[turn]
      if (existingClientRequestId && existingClientRequestId !== clientRequestId) {
        // The turn is already bound under a different client_request_id.
        // Treat this as an alias instead of a fatal conflict so late / alternate IDs
        // (e.g. runtime ack vs event stream) cannot break the UI binding layer.
        set((currentState) => ({
          boundTurnByClientRequestId: {
            ...currentState.boundTurnByClientRequestId,
            [clientRequestId]: turn,
          },
        }))

        return {
          alreadyBound: true,
          turn,
          cancelRequested: false,
          messagesToPersist: [],
        }
      }

      const pendingKey = resolvePendingRequestKeyForTurnBinding(state, clientRequestId)
      const pendingRequest = pendingKey
        ? state.pendingRequestsByClientRequestId[pendingKey]
        : undefined
      const pendingMessageIds = pendingKey
        ? state.pendingUserMessageIdsByClientRequestId[pendingKey] || []
        : []
      const pendingMessageIdSet = new Set(pendingMessageIds)
      const messagesToPersist = [] as AgentChatState['messages']

      const cancelRequested = pendingRequest?.status === 'cancel_requested'

      set((currentState) => {
        const nextMessages = currentState.messages.map((message) => {
          if (!pendingMessageIdSet.has(message.id)) {
            return message
          }

          const nextMessage = {
            ...message,
            turn,
          }
          messagesToPersist.push(nextMessage)
          return nextMessage
        })

        const nextPendingRequests = { ...currentState.pendingRequestsByClientRequestId }
        const nextPendingUserMessageIds = { ...currentState.pendingUserMessageIdsByClientRequestId }

        if (pendingKey) {
          delete nextPendingRequests[pendingKey]
          delete nextPendingUserMessageIds[pendingKey]
        }

        // Also clear any stale entries under the authoritative clientRequestId (best-effort).
        delete nextPendingRequests[clientRequestId]
        delete nextPendingUserMessageIds[clientRequestId]

        const nextBoundTurnByClientRequestId = {
          ...currentState.boundTurnByClientRequestId,
          [clientRequestId]: turn,
          ...(pendingKey && pendingKey !== clientRequestId ? { [pendingKey]: turn } : {}),
        }

        return {
          turn,
          ...reduceMarkTurnStarted(currentState, turn),
          messages: nextMessages,
          pendingRequestsByClientRequestId: nextPendingRequests,
          boundTurnByClientRequestId: nextBoundTurnByClientRequestId,
          pendingUserMessageIdsByClientRequestId: nextPendingUserMessageIds,
          clientRequestIdByTurnId: {
            ...currentState.clientRequestIdByTurnId,
            [turn]: clientRequestId,
          },
        }
      })

      return {
        alreadyBound: false,
        turn,
        cancelRequested,
        messagesToPersist,
      }
    },

    failPendingTurnRequest: ({ clientRequestId, removePendingMessages = true }) => {
      set((state) => {
        const pendingMessageIds = state.pendingUserMessageIdsByClientRequestId[clientRequestId] || []
        const pendingMessageIdSet = new Set(pendingMessageIds)
        const nextPendingRequests = { ...state.pendingRequestsByClientRequestId }
        delete nextPendingRequests[clientRequestId]

        const nextPendingUserMessageIds = {
          ...state.pendingUserMessageIdsByClientRequestId,
        }
        delete nextPendingUserMessageIds[clientRequestId]

        return {
          messages: removePendingMessages && pendingMessageIdSet.size > 0
            ? state.messages.filter((message) => !pendingMessageIdSet.has(message.id))
            : state.messages,
          pendingRequestsByClientRequestId: nextPendingRequests,
          pendingUserMessageIdsByClientRequestId: nextPendingUserMessageIds,
        }
      })
    },

    clearPendingTurnState: () => {
      clearPendingTurnStateLocal()
    },

    setTurnPhase: (turn, phase, options) =>
      set((state) => {
        const next = reduceSetTurnPhase(state, turn, phase, options)
        if (phase !== 'completed' && phase !== 'cancelled' && phase !== 'failed') {
          return next
        }

        const mergedState = {
          ...state,
          ...next,
        } as AgentChatState
        const committedTimelineByTurnId = commitTurnTimelineSnapshotLocal(mergedState, turn)
        if (!committedTimelineByTurnId) {
          return next
        }

        return {
          ...next,
          committedTimelineByTurnId,
        }
      }),

    appendTurnAnswerDelta: (turn, delta) =>
      set((state) => reduceAppendTurnAnswerDelta(state, turn, delta)),

    appendTurnThinkingDelta: (turn, delta) =>
      set((state) => reduceAppendTurnThinkingDelta(state, turn, delta)),

    markToolStepStarted: (turn, input) =>
      set((state) => reduceUpsertToolStepStarted(state, turn, input)),

    markToolStepProgress: (turn, input) =>
      set((state) => reduceUpsertToolStepProgress(state, turn, input)),

    markToolStepCompleted: (turn, input) =>
      set((state) => reduceUpsertToolStepCompleted(state, turn, input)),

    markWaitingForConfirmation: (turn, input) =>
      set((state) => reduceMarkWaitingForConfirmation(state, turn, input)),

    pushTurnEvent: (turn, event) => {
      set((state) => reducePushTurnEvent(state, turn, event))
    },

    commitTurnTimelineSnapshot: (turn) => {
      set((state) => {
        const next = commitTurnTimelineSnapshotLocal(state, turn)
        if (!next) {
          return {}
        }

        return {
          committedTimelineByTurnId: next,
        }
      })
    },

    setTurnFeedback: (turnId, rating) =>
      set((state) => ({
        turnFeedback: { ...state.turnFeedback, [turnId]: rating },
      })),

    getTurnViews: () => selectTurnViews(get()),

    applyTodoState: (todoState) => set({ todoState }),

    openAskUserRequest: (request) => set({ pendingAskUser: request }),

    resolveAskUserRequest: (callId, answers) => {
      const pending = get().pendingAskUser
      if (!pending || pending.callId !== callId) {
        set({ pendingAskUser: undefined })
        return
      }

      const sessionId = get().session_id
      const turn = pending.turn

      set((state) => {
        const thinkingState = reduceSetStateStatus(state, 'thinking')
        const turnState = reduceSetTurnPhase(
          {
            ...state,
            ...thinkingState,
          } as AgentChatState,
          turn,
          'synthesizing',
        )
        const stepState = reduceMarkAskUserStepAnswered(
          {
            ...state,
            ...thinkingState,
            ...turnState,
          } as AgentChatState,
          turn,
          {
            callId,
            answers,
          },
        )

        return {
          ...thinkingState,
          ...turnState,
          ...stepState,
          pendingAskUser: undefined,
          sessionRuntimeState: 'running',
          sessionCanContinue: false,
          sessionCanResume: false,
          sessionReadonlyReason: undefined,
        }
      })

      agentTurnResumeClient({
        session_id: sessionId,
        turn_id: turn,
        resume_input: {
          kind: 'askuser',
          answers,
        },
      }).catch((error) => {
        console.error('[agent-engine-v2] resume (askuser) failed:', error)
        const runtimeUnavailable = isResumeRuntimeUnavailableError(error)

        set((state) => {
          if (state.pendingAskUser && state.pendingAskUser.callId !== callId) {
            return {}
          }

          const currentStep = state.stepsByTurnId[turn]?.find((step) => step.callId === callId)
          if (currentStep?.status === 'success') {
            return {}
          }

          if (runtimeUnavailable) {
            const idleState = reduceSetStateStatus(state, 'idle')
            return {
              ...idleState,
              pendingAskUser: undefined,
              sessionRuntimeState: 'degraded',
              sessionCanContinue: false,
              sessionCanResume: false,
              sessionReadonlyReason: 'runtime_state_unavailable',
            }
          }

          const waitingState = reduceSetStateStatus(state, 'waiting_askuser')
          const turnState = reduceSetTurnPhase(
            {
              ...state,
              ...waitingState,
            } as AgentChatState,
            turn,
            'tool_running',
          )
          const stepState = reduceMarkWaitingForConfirmation(
            {
              ...state,
              ...waitingState,
              ...turnState,
            } as AgentChatState,
            turn,
            {
              callId,
              toolName: currentStep?.toolName ?? 'askuser',
              waitState: 'waiting_askuser',
            },
          )

          return {
            ...waitingState,
            ...turnState,
            ...stepState,
            pendingAskUser: pending,
            sessionRuntimeState: 'suspended_askuser',
            sessionCanContinue: false,
            sessionCanResume: true,
            sessionReadonlyReason: undefined,
          }
        })
      })
    },

    cancelAskUserRequest: (callId) => {
      const pending = get().pendingAskUser
      if (pending && pending.callId === callId) {
        set({ pendingAskUser: undefined })
        return
      }
      set({ pendingAskUser: undefined })
    },

    clearPendingAskUser: () => set({ pendingAskUser: undefined }),

    reset: () => {
      clearPersistedSessionController()
      set(initialState())
    },

    runChatTurn: (inputText, options: RunChatTurnOptions = {}) => runChatTurn(inputText, options),

    retryTurn: async (turn, options: RunChatTurnOptions = {}) => {
      const userMessage = get().messages
        .filter((message) => message.role === 'user' && message.turn === turn.turn)
        .at(-1)

      const prompt = userMessage?.content?.trim()
      if (!prompt) {
        throw new Error('E_AGENT_RETRY_INPUT_MISSING')
      }

      logUiMetric({
        sessionId: get().session_id,
        turnId: turn.turn,
        metric: 'turn_retry_attempt_count',
        value: 1,
        tags: { sourceTurn: turn.turn },
      })

      return get().runChatTurn(prompt, options)
    },

    retryStep: async (turnId, callId, options: RunChatTurnOptions = {}) => {
      const state = get()
      const step = state.stepsByTurnId[turnId]?.find((item) => item.callId === callId)
      if (!step) {
        throw new Error('E_AGENT_RETRY_STEP_NOT_FOUND')
      }

      const userMessage = state.messages
        .filter((message) => message.role === 'user' && message.turn === turnId)
        .at(-1)
      const prompt = userMessage?.content?.trim()
      if (!prompt) {
        throw new Error('E_AGENT_RETRY_INPUT_MISSING')
      }

      logUiMetric({
        sessionId: state.session_id,
        turnId,
        metric: 'step_retry_attempt_count',
        value: 1,
        tags: {
          callId,
          toolName: step.toolName,
        },
      })

      return state.runChatTurn(prompt, options)
    },
  }
})

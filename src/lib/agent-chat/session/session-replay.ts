import type { AgentLoopStopReason, AgentTodoState } from '@/agent/types'
import {
  createEmptyTodoState,
  type AgentPendingAskUserRequest,
} from '@/agent/types'
import {
  mapStructuredAskUserQuestions,
  parseAskUserQuestionnaire,
} from '../askuser'

import {
  normalizeTodoStateFromToolResultPayload,
} from '../todo'
import {
  extractToolPreviewRefs,
  parseToolTraceV2,
  toFaultDomain,
} from '../tool-trace'
import type { ChatToolTrace, ChatUiMessage } from '../types'
import type { AgentSessionEvent, AgentSessionMeta } from './session-types'

function normalizePayloadValue(value: unknown): Record<string, unknown> | undefined {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return undefined
  }
  return value as Record<string, unknown>
}

function toMaybeString(value: unknown) {
  return typeof value === 'string' && value.trim() ? value : undefined
}

function normalizeSkillFromEventPayload(payload?: Record<string, unknown>) {
  if (!payload || typeof payload !== 'object') {
    return undefined
  }

  if (payload.kind !== 'skill_enabled') {
    return undefined
  }

  const skill = toMaybeString(payload.skill_name)
  return skill || undefined
}

function restorePendingAskUserRequest(input: {
  turn: number
  ts: number
  payload?: Record<string, unknown>
}): AgentPendingAskUserRequest | undefined {
  const payload = input.payload
  if (!payload) {
    return undefined
  }

  const callId = toMaybeString(payload.call_id) || toMaybeString(payload.llm_call_id)
  if (!callId) {
    return undefined
  }

  let questions = mapStructuredAskUserQuestions(payload.questions)
  let questionnaire = toMaybeString(payload.questionnaire) || ''

  if ((!questions || questions.length === 0) && questionnaire) {
    const parsed = parseAskUserQuestionnaire(questionnaire)
    if (parsed.ok) {
      questions = parsed.questions
      questionnaire = parsed.questionnaire
    }
  }

  if (!questions || questions.length === 0) {
    return undefined
  }

  if (!questionnaire) {
    questionnaire = questions.map((question, index) => `${index + 1}. ${question.question}`).join('\n')
  }

  return {
    callId,
    turn: input.turn,
    questionnaire,
    questions,
    requestedAt: input.ts,
  }
}

function toMessageFromEvent(input: {
  event: AgentSessionEvent
  fallbackId: string
}): ChatUiMessage | null {
  if (input.event.type !== 'message') {
    return null
  }

  const payload = normalizePayloadValue(input.event.payload)
  const role = payload?.role

  if (role !== 'user' && role !== 'assistant' && role !== 'tool' && role !== 'system') {
    return null
  }

  const content = typeof payload?.content === 'string' ? payload.content : ''
  const id = toMaybeString(payload?.message_id) || input.fallbackId

  return {
    id,
    role,
    content,
    turn: input.event.turn,
    ts: input.event.ts,
    tool_name: toMaybeString(payload?.tool_name),
    tool_call_id: toMaybeString(payload?.tool_call_id),
  }
}

function toTraceFromEvent(event: AgentSessionEvent): ChatToolTrace | null {
  if (event.type !== 'tool_result') {
    return null
  }

  const payload = normalizePayloadValue(event.payload)
  const parsed = parseToolTraceV2(payload?.trace)
  if (!parsed) {
    return null
  }

  if (parsed.meta.tool.trim().toLowerCase() === 'todowrite') {
    return null
  }

  const traceError = parsed.result.error
  const refs = extractToolPreviewRefs(payload?.trace)

  return {
    turn: event.turn || 0,
    call_id: parsed.meta.call_id,
    tool_name: parsed.meta.tool,
    status: parsed.result.ok ? 'ok' : 'error',
    duration_ms: parsed.meta.duration_ms,
    fault_domain: toFaultDomain(traceError?.fault_domain),
    error_code: toMaybeString(traceError?.code),
    error_message: toMaybeString(traceError?.message),
    stage: parsed.stage,
    revision_before: parsed.meta.revision_before,
    revision_after: parsed.meta.revision_after,
    tx_id: parsed.meta.tx_id,
    preview: Object.keys(parsed.result.preview).length > 0 ? parsed.result.preview : undefined,
    refs: refs || undefined,
  }
}

function stopReasonFromType(type: AgentSessionEvent['type']): AgentLoopStopReason | undefined {
  if (type === 'turn_completed') return 'success'
  if (type === 'turn_cancelled') return 'cancel'
  if (type === 'turn_failed') return 'error'
  return undefined
}

function stopReasonFromPayload(payload: Record<string, unknown> | undefined): AgentLoopStopReason | undefined {
  const stopReason = payload?.stop_reason
  if (stopReason === 'success' || stopReason === 'cancel' || stopReason === 'error' || stopReason === 'limit') {
    return stopReason
  }

  return undefined
}

function hasNonMonotonicEventSeq(events: AgentSessionEvent[]) {
  let previousSeq = 0

  for (const event of events) {
    const seq = typeof event.event_seq === 'number' && Number.isFinite(event.event_seq) && event.event_seq > 0
      ? Math.floor(event.event_seq)
      : undefined

    if (!seq) {
      continue
    }

    if (seq <= previousSeq) {
      return true
    }

    previousSeq = seq
  }

  return false
}

function eventSortKey(event: AgentSessionEvent, index: number, forceFileOrder: boolean) {
  const seq = !forceFileOrder && typeof event.event_seq === 'number' && Number.isFinite(event.event_seq) && event.event_seq > 0
    ? Math.floor(event.event_seq)
    : index + 1
  const ts = typeof event.ts === 'number' && Number.isFinite(event.ts)
    ? event.ts
    : 0
  const id = typeof event.event_id === 'string' ? event.event_id : `legacy_${index}`

  return {
    seq,
    ts,
    id,
    index,
  }
}

function sortEventsForReplay(events: AgentSessionEvent[]) {
  const forceFileOrder = hasNonMonotonicEventSeq(events)

  return events
    .map((event, index) => ({ event, key: eventSortKey(event, index, forceFileOrder) }))
    .sort((a, b) => {
      if (a.key.seq !== b.key.seq) {
        return a.key.seq - b.key.seq
      }

      if (a.key.ts !== b.key.ts) {
        return a.key.ts - b.key.ts
      }

      if (a.key.id !== b.key.id) {
        return a.key.id.localeCompare(b.key.id)
      }

      return a.key.index - b.key.index
    })
    .map((item) => item.event)
}

export interface SessionReplayState {
  sessionId: string
  turn: number
  activeChapterPath?: string
  activeSkill?: string
  messages: ChatUiMessage[]
  traces: ChatToolTrace[]
  lastStopReason?: AgentLoopStopReason
  turnStopReasonById: Record<number, AgentLoopStopReason>
  todoState: AgentTodoState
  pendingAskUser?: AgentPendingAskUserRequest
  /** Last persisted turn_state (for pause/resume recovery) */
  lastTurnState?: { turn: number; state: string; payload?: Record<string, unknown> }
}

function replayStateFromEvents(input: {
  sessionId: string
  events: AgentSessionEvent[]
}): SessionReplayState {
  const answeredAskUserCallIds = new Set<string>()
  const state: SessionReplayState = {
    sessionId: input.sessionId,
    turn: 0,
    activeChapterPath: undefined,
    activeSkill: undefined,
    messages: [],
    traces: [],
    lastStopReason: undefined,
    turnStopReasonById: {},
    todoState: createEmptyTodoState(),
    pendingAskUser: undefined,
    lastTurnState: undefined,
  }

  const sortedEvents = sortEventsForReplay(input.events)

  for (let index = 0; index < sortedEvents.length; index += 1) {
    const event = sortedEvents[index]
    const turn = event.turn && event.turn > 0 ? event.turn : 0

    if (turn > state.turn) {
      state.turn = turn
    }

    const payload = normalizePayloadValue(event.payload)
    const activeChapterPath = toMaybeString(payload?.active_chapter_path)
    if (activeChapterPath) {
      state.activeChapterPath = activeChapterPath
    }

    const message = toMessageFromEvent({
      event,
      fallbackId: `replay_msg_${index}_${event.ts}`,
    })
    if (message) {
      state.messages.push(message)
    }

    const trace = toTraceFromEvent(event)
    if (trace) {
      state.traces.push(trace)
    }

    const todoStateFromToolResult = event.type === 'tool_result'
      ? normalizeTodoStateFromToolResultPayload(payload)
      : null
    if (todoStateFromToolResult) {
      state.todoState = todoStateFromToolResult
    }

    const skill = normalizeSkillFromEventPayload(payload)
    if (skill) {
      state.activeSkill = skill
    }

    if (event.type === 'turn_state' && payload) {
      const turnStateValue = toMaybeString(payload.state)
      if (turnStateValue && turn > 0) {
        state.lastTurnState = { turn, state: turnStateValue, payload }
        if (turnStateValue === 'waiting_confirmation') {
          state.turnStopReasonById[turn] = 'cancel'
        } else if (turnStateValue === 'waiting_askuser') {
          state.turnStopReasonById[turn] = 'cancel'
          const pendingAskUser = restorePendingAskUserRequest({
            turn,
            ts: event.ts,
            payload,
          })

          if (pendingAskUser && !answeredAskUserCallIds.has(pendingAskUser.callId)) {
            state.pendingAskUser = pendingAskUser
          }
        } else if (turnStateValue === 'resumed') {
          const resumedCallId = toMaybeString(payload.call_id)
            || toMaybeString(payload.pending_call_id)
            || state.pendingAskUser?.callId

          if (resumedCallId) {
            answeredAskUserCallIds.add(resumedCallId)
          }

          if (
            state.pendingAskUser?.turn === turn
            && (!resumedCallId || state.pendingAskUser.callId === resumedCallId)
          ) {
            state.pendingAskUser = undefined
          }
        }
      }
     }

    const reason = stopReasonFromPayload(payload) || stopReasonFromType(event.type)
    if (reason && turn > 0) {
      state.turnStopReasonById[turn] = reason
      state.lastStopReason = reason
      if (state.pendingAskUser?.turn === turn) {
        state.pendingAskUser = undefined
      }
    }
  }

  return state
}

export function replaySessionState(input: {
  sessionId: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
}): SessionReplayState {
  const replay = replayStateFromEvents({
    sessionId: input.sessionId,
    events: input.events,
  })

  if (input.meta?.active_chapter_path) {
    replay.activeChapterPath = input.meta.active_chapter_path
  }

  if (typeof input.meta?.last_turn === 'number' && input.meta.last_turn > replay.turn) {
    replay.turn = input.meta.last_turn
  }

  if (input.meta?.last_stop_reason) {
    replay.lastStopReason = input.meta.last_stop_reason
    if (typeof input.meta?.last_turn === 'number' && input.meta.last_turn > 0) {
      replay.turnStopReasonById[input.meta.last_turn] = input.meta.last_stop_reason
    }
  }

  return replay
}

import type { AgentCompactionMeta, AgentLoopStopReason } from '@/agent/types'

import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnPhase,
  AgentUiTurnState,
  ChatToolTrace,
  ChatUiMessage,
  OpenAiMessage,
} from '../types'

import { buildReviewCheckPreview, redactValue, trimRawOutput } from '../tool-step-utils'

function asTurnNumber(value: number | undefined) {
  return typeof value === 'number' && value > 0 ? value : undefined
}

export function collectTurns(
  messages: ChatUiMessage[],
  traces: ChatToolTrace[],
  stopReasons: Record<number, AgentLoopStopReason>,
) {
  const turns = new Set<number>()

  for (const key of Object.keys(stopReasons)) {
    const turn = Number(key)
    if (Number.isFinite(turn) && turn > 0) {
      turns.add(turn)
    }
  }

  for (const message of messages) {
    const turn = asTurnNumber(message.turn)
    if (turn) {
      turns.add(turn)
    }
  }

  for (const trace of traces) {
    const turn = asTurnNumber(trace.turn)
    if (turn) {
      turns.add(turn)
    }
  }

  return [...turns].sort((a, b) => a - b)
}

function toTurnPhase(stopReason: AgentLoopStopReason | undefined): AgentUiTurnPhase {
  if (stopReason === 'cancel') return 'cancelled'
  if (stopReason === 'error') return 'failed'
  return 'completed'
}

function buildTurnState(input: {
  turn: number
  messages: ChatUiMessage[]
  stopReason?: AgentLoopStopReason
}): AgentUiTurnState {
  const timestamps = input.messages
    .map((item) => item.ts)
    .filter((value) => typeof value === 'number' && Number.isFinite(value))

  const startedAt = timestamps.length > 0 ? Math.min(...timestamps) : Date.now()
  const updatedAt = timestamps.length > 0 ? Math.max(...timestamps) : startedAt
  const phase = toTurnPhase(input.stopReason)

  return {
    turn: input.turn,
    phase,
    startedAt,
    updatedAt,
    finishedAt: phase === 'completed' || phase === 'cancelled' || phase === 'failed' ? updatedAt : undefined,
    stopReason: input.stopReason,
  }
}

export function buildToolCallIdByTurn(messages: ChatUiMessage[], traces: ChatToolTrace[]) {
  const result: Record<number, string[]> = {}

  const append = (turn: number, callId: string) => {
    if (!result[turn]) {
      result[turn] = []
    }

    if (!result[turn].includes(callId)) {
      result[turn].push(callId)
    }
  }

  for (const trace of traces) {
    const turn = asTurnNumber(trace.turn)
    if (!turn) {
      continue
    }

    if (!trace.call_id) {
      continue
    }

    append(turn, trace.call_id)
  }

  for (const message of messages) {
    if (message.role !== 'tool') {
      continue
    }

    const turn = asTurnNumber(message.turn)
    if (!turn) {
      continue
    }

    if (message.tool_call_id) {
      append(turn, message.tool_call_id)
    }
  }

  return result
}

export function toOpenAiMessage(
  message: ChatUiMessage,
  input?: {
    toolCallIdByTurn?: Record<number, string[]>
    usedToolCallIdCountByTurn?: Record<number, number>
  },
): OpenAiMessage | null {
  if (message.role === 'tool') {
    const turn = asTurnNumber(message.turn)
    const candidates = turn ? (input?.toolCallIdByTurn?.[turn] || []) : []
    const usedCount = turn ? (input?.usedToolCallIdCountByTurn?.[turn] || 0) : 0
    const fallbackToolCallId = candidates[usedCount]
    const toolCallId = message.tool_call_id || fallbackToolCallId

    if (!toolCallId) {
      return null
    }

    if (turn && input?.usedToolCallIdCountByTurn) {
      input.usedToolCallIdCountByTurn[turn] = usedCount + 1
    }

    return {
      role: 'tool',
      tool_call_id: toolCallId,
      name: message.tool_name,
      content: message.content,
    }
  }

  return {
    role: message.role,
    content: message.content,
  }
}

function toToolStep(trace: ChatToolTrace, index: number): AgentUiToolStep {
  const status = trace.status === 'ok' ? 'success' : 'error'
  const startedAt = Date.now() + index

  if (trace.tool_name === 'review_check') {
    const preview = buildReviewCheckPreview(trace.preview) || (trace.preview ? (redactValue(trace.preview) as Record<string, unknown>) : null)
    const previewRecord = preview && typeof preview === 'object' && !Array.isArray(preview)
      ? (preview as Record<string, unknown>)
      : null

    const overall = typeof previewRecord?.overall_status === 'string' ? previewRecord.overall_status : undefined
    const counts = (previewRecord && typeof previewRecord.issue_counts === 'object' && previewRecord.issue_counts && !Array.isArray(previewRecord.issue_counts))
      ? (previewRecord.issue_counts as Record<string, unknown>)
      : null
    const block = typeof counts?.block === 'number' ? counts.block : undefined
    const warn = typeof counts?.warn === 'number' ? counts.warn : undefined
    const action = typeof previewRecord?.recommended_action === 'string' ? previewRecord.recommended_action : undefined

    const summaryParts = [
      typeof block === 'number' ? `block=${block}` : null,
      typeof warn === 'number' ? `warn=${warn}` : null,
      action ? `action=${action}` : null,
    ].filter(Boolean)
    const resultSummary = summaryParts.length
      ? `review ${overall || trace.status} · ${summaryParts.join(', ')}`
      : `review ${overall || trace.status}`

    return {
      callId: trace.call_id,
      toolName: trace.tool_name,
      status,
      startedAt,
      finishedAt: startedAt + trace.duration_ms,
      durationMs: trace.duration_ms,
      resultSummary,
      summary: trace.error_message || resultSummary,
      errorMessage: trace.error_message,
      errorCode: trace.error_code,
      faultDomain: trace.fault_domain,
      stage: trace.stage,
      revisionBefore: trace.revision_before,
      revisionAfter: trace.revision_after,
      txId: trace.tx_id,
      retryable: trace.status === 'error',
      outputPreview: preview ?? undefined,
      rawOutput: trace.preview ? trimRawOutput(JSON.stringify(trace.preview)) : undefined,
    }
  }

  return {
    callId: trace.call_id,
    toolName: trace.tool_name,
    status,
    startedAt,
    finishedAt: startedAt + trace.duration_ms,
    durationMs: trace.duration_ms,
    resultSummary: `${trace.tool_name} ${trace.status}`,
    summary: trace.error_message || `${trace.tool_name} ${trace.status}`,
    errorMessage: trace.error_message,
    errorCode: trace.error_code,
    faultDomain: trace.fault_domain,
    stage: trace.stage,
    revisionBefore: trace.revision_before,
    revisionAfter: trace.revision_after,
    txId: trace.tx_id,
    retryable: trace.status === 'error',
  }
}

export function buildToolStepsByTurn(traces: ChatToolTrace[]) {
  const grouped: Record<number, AgentUiToolStep[]> = {}

  traces.forEach((trace, index) => {
    const turn = asTurnNumber(trace.turn)
    if (!turn) {
      return
    }

    const current = grouped[turn] || []
    grouped[turn] = [...current, toToolStep(trace, index)]
  })

  return grouped
}

export function buildAnswersByTurn(messages: ChatUiMessage[]) {
  const grouped: Record<number, string> = {}

  for (const message of messages) {
    if (message.role !== 'assistant') {
      continue
    }

    const turn = asTurnNumber(message.turn)
    if (!turn) {
      continue
    }

    grouped[turn] = `${grouped[turn] || ''}${message.content}`
  }

  return grouped
}

export function buildTurnStateMaps(input: {
  turnOrder: number[]
  messages: ChatUiMessage[]
  turnStopReasonById: Record<number, AgentLoopStopReason>
}) {
  const turnById: Record<number, AgentUiTurnState> = {}

  for (const turn of input.turnOrder) {
    const turnMessages = input.messages.filter((item) => item.turn === turn)
    const stopReason = input.turnStopReasonById[turn]

    turnById[turn] = buildTurnState({
      turn,
      messages: turnMessages,
      stopReason,
    })
  }

  return {
    turnById,
  }
}

function toTimelineType(value: unknown): AgentUiTimelineEvent['type'] | null {
  const type = String(value || '')
  switch (type) {
    case 'TURN_STARTED':
    case 'PLAN_STARTED':
    case 'STREAMING_STARTED':
    case 'ASSISTANT_TEXT_DELTA':
    case 'THINKING_TEXT_DELTA':
    case 'TOOL_CALL_STARTED':
    case 'TOOL_CALL_PROGRESS':
    case 'TOOL_CALL_FINISHED':
    case 'WAITING_FOR_CONFIRMATION':
    case 'ASKUSER_REQUESTED':
    case 'ASKUSER_ANSWERED':
    case 'SYNTHESIS_STARTED':
    case 'COMPACTION_STARTED':
    case 'COMPACTION_FINISHED':
    case 'COMPACTION_FALLBACK':
    case 'TURN_COMPLETED':
    case 'TURN_CANCELLED':
    case 'TURN_FAILED':
      return type
    default:
      return null
  }
}

function isTodoWritePayload(payload: Record<string, unknown>) {
  const toolName = typeof payload.tool_name === 'string' ? payload.tool_name.trim().toLowerCase() : ''
  return toolName === 'todowrite'
}

function timelineTypeFromSessionEvent(input: import('./session-types').AgentSessionEvent): AgentUiTimelineEvent['type'] | null {
  if (input.type === 'turn_started') {
    return 'TURN_STARTED'
  }

  if (input.type === 'tool_execution') {
    const payload = normalizeTimelineEventPayload(input.payload)
    if (isTodoWritePayload(payload)) {
      return null
    }
    return 'TOOL_CALL_STARTED'
  }

  if (input.type === 'tool_result') {
    const payload = normalizeTimelineEventPayload(input.payload)
    if (isTodoWritePayload(payload)) {
      return null
    }
    return 'TOOL_CALL_FINISHED'
  }

  if (input.type === 'compaction_summary') {
    return 'COMPACTION_FINISHED'
  }

  if (input.type === 'compaction_started') {
    return 'COMPACTION_STARTED'
  }

  if (input.type === 'compaction_finished') {
    return 'COMPACTION_FINISHED'
  }

  if (input.type === 'compaction_fallback') {
    return 'COMPACTION_FALLBACK'
  }

  if (input.type === 'turn_cancelled') {
    return 'TURN_CANCELLED'
  }

  if (input.type === 'turn_failed') {
    return 'TURN_FAILED'
  }

  if (input.type === 'turn_completed') {
    const payload = normalizeTimelineEventPayload(input.payload)
    const stopReason = payload.stop_reason
    if (stopReason === 'cancel') {
      return 'TURN_CANCELLED'
    }
    if (stopReason === 'error') {
      return 'TURN_FAILED'
    }
    return 'TURN_COMPLETED'
  }

  return null
}

function eventSortKey(event: import('./session-types').AgentSessionEvent, index: number) {
  const seq = typeof event.event_seq === 'number' && Number.isFinite(event.event_seq) && event.event_seq > 0
    ? Math.floor(event.event_seq)
    : index + 1
  const ts = typeof event.ts === 'number' && Number.isFinite(event.ts)
    ? event.ts
    : 0
  const id = typeof event.event_id === 'string' ? event.event_id : `timeline_${index}`

  return {
    seq,
    ts,
    id,
    index,
  }
}

function normalizeTimelineEventPayload(payload: unknown) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return {}
  }

  return payload as Record<string, unknown>
}

function extractTimelineEventMeta(
  event: import('./session-types').AgentSessionEvent,
  timelineType: AgentUiTimelineEvent['type'],
  payload: Record<string, unknown>,
) {
  if (event.type === 'compaction_summary' || event.type === 'compaction_fallback') {
    return payload
  }

  const hasToolExposure = typeof payload.tool_package === 'string'
    || typeof payload.route_reason === 'string'
    || typeof payload.fallback_from === 'string'
    || Array.isArray(payload.exposed_tools)
    || Array.isArray(payload.skipped_tools)

  if (hasToolExposure && (
    timelineType === 'PLAN_STARTED'
    || timelineType === 'TURN_STARTED'
    || timelineType === 'TURN_COMPLETED'
    || timelineType === 'TURN_FAILED'
  )) {
    return payload
  }

  return undefined
}

function inferTurnStartedAtMap(input: {
  events: import('./session-types').AgentSessionEvent[]
  replayedAt?: number
}) {
  const startedAtByTurn: Record<number, number> = {}

  for (const event of input.events) {
    const turn = asTurnNumber(event.turn)
    if (!turn) {
      continue
    }

    const ts = typeof event.ts === 'number' && Number.isFinite(event.ts)
      ? event.ts
      : undefined

    if (ts === undefined) {
      continue
    }

    const existing = startedAtByTurn[turn]
    startedAtByTurn[turn] = typeof existing === 'number'
      ? Math.min(existing, ts)
      : ts
  }

  if (typeof input.replayedAt === 'number' && Number.isFinite(input.replayedAt)) {
    for (const event of input.events) {
      const turn = asTurnNumber(event.turn)
      if (!turn) {
        continue
      }

      if (typeof startedAtByTurn[turn] !== 'number') {
        startedAtByTurn[turn] = input.replayedAt
      }
    }
  }

  return startedAtByTurn
}

export function buildTimelineEventsByTurn(events: import('./session-types').AgentSessionEvent[], replayedAt?: number) {
  const grouped: Record<number, AgentUiTimelineEvent[]> = {}
  const startedAtByTurn = inferTurnStartedAtMap({ events, replayedAt })

  const sortedEvents = events
    .map((event, index) => ({ event, key: eventSortKey(event, index) }))
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

  for (let index = 0; index < sortedEvents.length; index += 1) {
    const event = sortedEvents[index]
    const turn = asTurnNumber(event.turn)
    if (!turn) {
      continue
    }

    const payload = normalizeTimelineEventPayload(event.payload)
    const timelineType = toTimelineType(payload.timeline_type) || timelineTypeFromSessionEvent(event)
    if (!timelineType) {
      continue
    }

    const list = grouped[turn] || []
    const stopReason = typeof payload.stop_reason === 'string' ? payload.stop_reason : undefined
    const compactionSummary = typeof payload.summary_text === 'string'
      ? payload.summary_text
      : undefined
    const fallbackTsBase = startedAtByTurn[turn] ?? Date.now()
    const ts = typeof event.ts === 'number' && Number.isFinite(event.ts)
      ? event.ts
      : fallbackTsBase + index
    const seq = typeof event.event_seq === 'number' && Number.isFinite(event.event_seq) && event.event_seq > 0
      ? Math.floor(event.event_seq)
      : index + 1

    const nextEvent: AgentUiTimelineEvent = {
      id: typeof event.event_id === 'string' && event.event_id.trim()
        ? event.event_id
        : typeof payload.event_id === 'string' && payload.event_id.trim()
          ? payload.event_id
          : `replay_evt_${turn}_${ts}_${index}`,
      turn,
      type: timelineType,
      ts,
      seq,
      summary: typeof payload.summary === 'string'
        ? payload.summary
        : compactionSummary
          ? `compaction: ${compactionSummary.slice(0, 80)}`
          : stopReason
            ? `turn ${stopReason}`
            : undefined,
      callId: typeof payload.call_id === 'string' ? payload.call_id : undefined,
      delta: typeof payload.delta === 'string' ? payload.delta : undefined,
      meta: extractTimelineEventMeta(event, timelineType, payload),
    }

    grouped[turn] = [...list, nextEvent]
  }

  return grouped
}

function toMaybeNumber(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function toMaybeString(value: unknown) {
  return typeof value === 'string' && value.trim() ? value : undefined
}

function toStringArray(value: unknown) {
  if (!Array.isArray(value)) {
    return []
  }

  return value
    .map((item) => String(item ?? '').trim())
    .filter(Boolean)
}

function toCompactionMeta(event: import('./session-types').AgentSessionEvent): AgentCompactionMeta | undefined {
  if (event.type !== 'compaction_summary') {
    return undefined
  }

  const payload = normalizeTimelineEventPayload(event.payload)
  const strategy = payload.strategy
  if (strategy !== 'threshold' && strategy !== 'context_limit') {
    return undefined
  }

  const summaryText = toMaybeString(payload.summary_text) || ''
  if (!summaryText) {
    return undefined
  }

  const sourceWindow = normalizeTimelineEventPayload(payload.source_window)
  const startIndex = toMaybeNumber(sourceWindow.start_index)
  const endIndex = toMaybeNumber(sourceWindow.end_index)
  if (startIndex === undefined || endIndex === undefined) {
    return undefined
  }

  const anchor = normalizeTimelineEventPayload(payload.anchor)

  return {
    strategy,
    summary_text: summaryText,
    anchors: toStringArray(payload.anchors),
    removed_count: toMaybeNumber(payload.removed_count) ?? 0,
    keep_recent_count: toMaybeNumber(payload.keep_recent_count) ?? 0,
    source_window: {
      start_index: startIndex,
      end_index: endIndex,
    },
    anchor: toMaybeNumber(anchor.anchor_index) !== undefined
      ? {
        anchor_index: toMaybeNumber(anchor.anchor_index) ?? 0,
        anchor_preview: toMaybeString(anchor.anchor_preview),
      }
      : undefined,
    reason: toMaybeString(payload.reason),
  }
}

export function extractLastCompaction(events: import('./session-types').AgentSessionEvent[]) {
  let latest: AgentCompactionMeta | undefined

  for (const event of events) {
    const meta = toCompactionMeta(event)
    if (meta) {
      latest = meta
    }
  }

  return latest
}

export function buildThinkingByTurn(eventsByTurnId: Record<number, AgentUiTimelineEvent[]>) {
  const thinkingByTurn: Record<number, string> = {}

  for (const [key, events] of Object.entries(eventsByTurnId)) {
    const turn = Number(key)
    if (!Number.isFinite(turn)) {
      continue
    }

    const thinkingText = events
      .filter((event) => event.type === 'THINKING_TEXT_DELTA')
      .map((event) => event.delta ?? '')
      .join('')

    if (thinkingText) {
      thinkingByTurn[turn] = thinkingText
    }
  }

  return thinkingByTurn
}

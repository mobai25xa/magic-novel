import type {
  ChatToolTrace,
  ChatUiMessage,
} from '../types'
import {
  AGENT_SESSION_SCHEMA_VERSION,
  type AgentSessionEventDiagnostics,
  type AgentSessionEvent,
} from './session-types'

function appendEventDiagnostics(input: {
  turn?: number
  payload: Record<string, unknown>
  diagnostics?: AgentSessionEventDiagnostics
}) {
  return {
    ...input.payload,
    ...(typeof input.turn === 'number' && input.turn > 0 ? { bound_turn_id: input.turn } : {}),
    ...(typeof input.diagnostics?.bound_turn_id === 'number' && input.diagnostics.bound_turn_id > 0
      ? { bound_turn_id: input.diagnostics.bound_turn_id }
      : {}),
    ...(typeof input.diagnostics?.client_request_id === 'string' && input.diagnostics.client_request_id.trim()
      ? { client_request_id: input.diagnostics.client_request_id }
      : {}),
    ...(typeof input.diagnostics?.hydrate_source === 'string' && input.diagnostics.hydrate_source.trim()
      ? { hydrate_source: input.diagnostics.hydrate_source }
      : {}),
  }
}

function buildEvent(input: {
  sessionId: string
  type: AgentSessionEvent['type']
  turn?: number
  payload?: Record<string, unknown>
  ts?: number
}): AgentSessionEvent {
  return {
    schema_version: AGENT_SESSION_SCHEMA_VERSION,
    type: input.type,
    session_id: input.sessionId,
    ts: input.ts ?? Date.now(),
    turn: input.turn,
    payload: input.payload,
  }
}

function messagePayload(message: ChatUiMessage): Record<string, unknown> {
  return {
    role: message.role,
    content: message.content,
    tool_name: message.tool_name,
    tool_call_id: message.tool_call_id,
    message_id: message.id,
  }
}

function toolTracePayload(trace: ChatToolTrace): Record<string, unknown> {
  const error = trace.status === 'error'
    ? {
      code: trace.error_code,
      message: trace.error_message,
      retryable: false,
      fault_domain: trace.fault_domain,
    }
    : null

  return {
    schema_version: 2,
    stage: trace.stage || 'result',
    meta: {
      tool: trace.tool_name,
      call_id: trace.call_id,
      duration_ms: trace.duration_ms,
      revision_before: trace.revision_before,
      revision_after: trace.revision_after,
      tx_id: trace.tx_id,
    },
    result: {
      ok: trace.status === 'ok',
      preview: trace.preview || {},
      error,
    },
    refs: trace.refs || {},
  }
}

export function toSessionMessageEvent(input: {
  sessionId: string
  message: ChatUiMessage
  diagnostics?: AgentSessionEventDiagnostics
}): AgentSessionEvent {
  return buildEvent({
    sessionId: input.sessionId,
    type: 'message',
    turn: input.message.turn,
    payload: appendEventDiagnostics({
      turn: input.message.turn,
      payload: messagePayload(input.message),
      diagnostics: input.diagnostics,
    }),
    ts: input.message.ts,
  })
}

export function toSessionToolResultEvent(input: {
  sessionId: string
  turnId: number
  trace: ChatToolTrace
  ts?: number
  diagnostics?: AgentSessionEventDiagnostics
}): AgentSessionEvent {
  return buildEvent({
    sessionId: input.sessionId,
    type: 'tool_result',
    turn: input.turnId,
    payload: appendEventDiagnostics({
      turn: input.turnId,
      payload: {
        call_id: input.trace.call_id,
        tool_name: input.trace.tool_name,
        status: input.trace.status,
        trace: toolTracePayload(input.trace),
      },
      diagnostics: input.diagnostics,
    }),
    ts: input.ts,
  })
}

export function toSessionTurnFinalEvent(input: {
  sessionId: string
  turnId: number
  stopReason: 'success' | 'cancel' | 'error' | 'limit'
  latencyMs?: number
  ts?: number
  diagnostics?: AgentSessionEventDiagnostics
}): AgentSessionEvent {
  const type = input.stopReason === 'cancel'
    ? 'turn_cancelled'
    : input.stopReason === 'error'
      ? 'turn_failed'
      : 'turn_completed'

  return buildEvent({
    sessionId: input.sessionId,
    type,
    turn: input.turnId,
    payload: appendEventDiagnostics({
      turn: input.turnId,
      payload: {
        stop_reason: input.stopReason,
        latency_ms: input.latencyMs,
      },
      diagnostics: input.diagnostics,
    }),
    ts: input.ts,
  })
}

export function toSessionTurnStateEvent(input: {
  sessionId: string
  turnId: number
  state: 'waiting_confirmation' | 'waiting_askuser' | 'paused' | 'resumed'
  payload?: Record<string, unknown>
  ts?: number
  diagnostics?: AgentSessionEventDiagnostics
}): AgentSessionEvent {
  return buildEvent({
    sessionId: input.sessionId,
    type: 'turn_state',
    turn: input.turnId,
    payload: appendEventDiagnostics({
      turn: input.turnId,
      payload: {
        state: input.state,
        ...input.payload,
      },
      diagnostics: input.diagnostics,
    }),
    ts: input.ts,
  })
}

export function toSessionCompactionStartedEvent(input: {
  sessionId: string
  turnId: number
  reason: string
  ts?: number
}): AgentSessionEvent {
  return buildEvent({
    sessionId: input.sessionId,
    type: 'compaction_started',
    turn: input.turnId,
    payload: { reason: input.reason },
    ts: input.ts,
  })
}

export function toSessionCompactionFinishedEvent(input: {
  sessionId: string
  turnId: number
  meta?: Record<string, unknown>
  ts?: number
}): AgentSessionEvent {
  return buildEvent({
    sessionId: input.sessionId,
    type: 'compaction_finished',
    turn: input.turnId,
    payload: { meta: input.meta },
    ts: input.ts,
  })
}

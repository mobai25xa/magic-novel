import type { AgentToolTrace } from '@/agent/types'

import { normalizeTodoStateFromToolResultPayload } from '../../todo'
import {
  extractToolPreviewRefs,
  parseToolTraceV2,
  toFaultDomain,
} from '../../tool-trace'

import { buildToolRefreshChangeSet, scheduleToolRefresh } from '../tool-refresh'
import { parseArgsPreview, toToolTraceStage } from '../utils'
import { enqueueToolStepProgress, flushBatchedRuntimeUpdates } from '../batched-runtime-updates'

import type { AgentRuntimeEventContext } from '../agent-event-context'

export function handleToolEvent(ctx: AgentRuntimeEventContext) {
  switch (ctx.envelope.type) {
    case 'TOOL_CALL_STARTED':
      handleToolCallStarted(ctx)
      break
    case 'TOOL_CALL_PROGRESS':
      handleToolCallProgress(ctx)
      break
    case 'TOOL_CALL_FINISHED':
      handleToolCallFinished(ctx)
      break
    case 'WAITING_FOR_CONFIRMATION':
      handleWaitingForConfirmation(ctx)
      break
    default:
      break
  }
}

function handleToolCallStarted(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx
  const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
  const toolName = String(payload.tool_name ?? '')
  const normalizedToolName = toolName.trim().toLowerCase()

  if (normalizedToolName === 'todowrite') {
    return
  }

  store.setStateStatus('executing_tool')
  store.markToolStepStarted(turn, {
    callId,
    llmCallId: payload.llm_call_id as string | undefined,
    toolName,
    args: parseArgsPreview(payload.args_preview),
    ts,
  })
  store.pushTurnEvent(turn, {
    type: 'TOOL_CALL_STARTED',
    ts,
    callId,
    summary: `${toolName} · started`,
  })
}

function handleToolCallProgress(ctx: AgentRuntimeEventContext) {
  const { turn, ts, payload } = ctx
  const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
  const toolName = String(payload.tool_name ?? '')
  const normalizedToolName = toolName.trim().toLowerCase()

  if (normalizedToolName === 'todowrite') {
    return
  }

  enqueueToolStepProgress({
    sessionId: ctx.envelope.session_id,
    turn,
    progress: {
      callId,
      llmCallId: payload.llm_call_id as string | undefined,
      toolName,
      progress: String(payload.progress ?? 'running'),
      ts,
    },
  })
}

function handleToolCallFinished(ctx: AgentRuntimeEventContext) {
  flushBatchedRuntimeUpdates({ sessionId: ctx.envelope.session_id, turn: ctx.turn })

  const { store, turn, ts, payload } = ctx

  const parsedTrace = parseToolTraceV2(payload.trace)
  const callId = String(parsedTrace?.meta.call_id ?? payload.call_id ?? payload.llm_call_id ?? '')
  const toolName = String(parsedTrace?.meta.tool ?? payload.tool_name ?? '')
  const normalizedToolName = toolName.trim().toLowerCase()
  const status = parsedTrace ? (parsedTrace.result.ok ? 'ok' : 'error') : String(payload.status ?? 'ok')
  const traceError = parsedTrace?.result.error || null
  const traceFaultDomain = toFaultDomain(traceError?.fault_domain)
  const traceStage = parsedTrace?.stage || toToolTraceStage(payload.stage)
  const tracePreview = parsedTrace?.result.preview || {}
  const traceRefs = extractToolPreviewRefs(payload.trace)

  if (normalizedToolName === 'todowrite') {
    if (status === 'ok' && typeof store.applyTodoState === 'function') {
      const todoState = normalizeTodoStateFromToolResultPayload(payload)
      if (todoState) {
        store.applyTodoState(todoState)
      }
    }
    return
  }

  const completedTrace: AgentToolTrace = {
    turn,
    call_id: callId,
    tool_name: toolName,
    status: status === 'ok' ? 'ok' : 'error',
    duration_ms: parsedTrace?.meta.duration_ms
      ?? (typeof payload.duration_ms === 'number' ? payload.duration_ms : 0),
    fault_domain: traceFaultDomain,
    error_code: typeof traceError?.code === 'string' ? traceError.code : undefined,
    error_message: typeof traceError?.message === 'string'
      ? traceError.message
      : undefined,
    stage: traceStage,
    revision_before: parsedTrace?.meta.revision_before,
    revision_after: parsedTrace?.meta.revision_after,
    tx_id: parsedTrace?.meta.tx_id,
    preview: Object.keys(tracePreview).length > 0 ? tracePreview : undefined,
    refs: traceRefs || undefined,
  }

  store.markToolStepCompleted(turn, {
    callId,
    llmCallId: payload.llm_call_id as string | undefined,
    toolName,
    output: JSON.stringify(parsedTrace ?? payload.trace ?? {}),
    trace: completedTrace,
    ts,
  })
  store.pushTurnEvent(turn, {
    type: 'TOOL_CALL_FINISHED',
    ts,
    callId,
    summary: `${toolName} · ${status}`,
  })

  const refreshChangeSet = buildToolRefreshChangeSet({
    toolName,
    status,
    payload,
    tracePreview,
    traceRefs,
  })
  if (refreshChangeSet) {
    scheduleToolRefresh(refreshChangeSet)
  }
}

function handleWaitingForConfirmation(ctx: AgentRuntimeEventContext) {
  flushBatchedRuntimeUpdates({ sessionId: ctx.envelope.session_id, turn: ctx.turn })

  const { store, turn, ts, payload } = ctx
  const callId = String(payload.call_id ?? payload.llm_call_id ?? '')
  const toolName = String(payload.tool_name ?? '')
  store.setStateStatus('waiting_confirmation')
  // Only unlock resume after TURN_COMPLETED confirms the loop is fully suspended.
  store.markWaitingForConfirmation(turn, {
    callId,
    llmCallId: payload.llm_call_id as string | undefined,
    toolName,
    waitState: 'waiting_confirmation',
    ts,
  })
  store.pushTurnEvent(turn, {
    type: 'WAITING_FOR_CONFIRMATION',
    ts,
    callId,
    summary: `${toolName} · waiting confirmation: ${String(payload.reason ?? '')}`,
  })
}

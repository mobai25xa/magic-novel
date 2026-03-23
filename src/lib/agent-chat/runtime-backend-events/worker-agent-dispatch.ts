import { parseToolTraceV2 } from '../tool-trace'

import type { AgentEventEnvelope } from './types'

import { commitMissionUiState, getOrCreateMissionUiState, upsertMissionWorkerStatus } from './mission-store'

function extractWorkerFeatureIdFromPayload(payload: Record<string, unknown>) {
  const featureId = payload.feature_id
  if (typeof featureId === 'string' && featureId.trim()) {
    return featureId.trim()
  }
  return undefined
}

function summarizeTurnStarted(payload: Record<string, unknown>, turn: number) {
  const provider = String(payload.model_provider ?? '')
  const model = String(payload.model ?? '')
  const modelHint = provider || model ? `${provider}/${model}`.replace(/^\//, '').replace(/\/$/, '') : ''
  return {
    status: 'running',
    summary: modelHint ? `turn ${turn} · ${modelHint}` : `turn ${turn} · started`,
  }
}

function summarizeToolCallStarted(payload: Record<string, unknown>) {
  const toolName = String(payload.tool_name ?? '').trim()
  if (!toolName) return null
  if (toolName.toLowerCase() === 'todowrite') return null
  return { status: 'running', summary: `${toolName} · started` }
}

function summarizeToolCallProgress(payload: Record<string, unknown>) {
  const toolName = String(payload.tool_name ?? '').trim()
  if (!toolName) return null
  if (toolName.toLowerCase() === 'todowrite') return null
  const progress = String(payload.progress ?? 'running')
  return { status: 'running', summary: `${toolName} · ${progress}` }
}

function summarizeToolCallFinished(payload: Record<string, unknown>) {
  const parsedTrace = parseToolTraceV2(payload.trace)
  const toolName = String(parsedTrace?.meta.tool ?? payload.tool_name ?? '').trim()
  if (!toolName) return null
  if (toolName.toLowerCase() === 'todowrite') return null
  const ok = parsedTrace ? parsedTrace.result.ok : String(payload.status ?? 'ok') === 'ok'
  return { status: 'running', summary: `${toolName} · ${ok ? 'ok' : 'error'}` }
}

function summarizeWaitingForConfirmation(payload: Record<string, unknown>) {
  const toolName = String(payload.tool_name ?? '').trim() || 'tool'
  return { status: 'running', summary: `${toolName} · waiting confirmation` }
}

function summarizeTurnCompleted(payload: Record<string, unknown>, turn: number) {
  const rawStopReason = String(payload.stop_reason ?? 'success')
  if (rawStopReason === 'cancel') {
    return { status: 'cancelled', summary: `turn ${turn} · cancelled` }
  }
  if (rawStopReason === 'error') {
    return { status: 'failed', summary: `turn ${turn} · error` }
  }
  if (rawStopReason === 'waiting_confirmation' || rawStopReason === 'waiting_askuser') {
    return { status: 'running', summary: `turn ${turn} · ${rawStopReason}` }
  }
  return { status: 'completed', summary: `turn ${turn} · ${rawStopReason}` }
}

function summarizeTurnFailed(payload: Record<string, unknown>, turn: number) {
  const errorCode = String(payload.error_code ?? '')
  const errorMsg = String(payload.error_message ?? payload.error_code ?? 'unknown error')
  if (errorCode === 'E_CANCELLED') {
    return { status: 'cancelled', summary: `turn ${turn} · cancelled` }
  }
  return { status: 'failed', summary: `turn ${turn} · ${errorMsg}` }
}

function summarizeWorkerAgentEvent(envelope: AgentEventEnvelope): { status: string; summary?: string } | null {
  const payload = envelope.payload
  const turn = envelope.turn_id

  switch (envelope.type) {
    case 'TURN_STARTED':
      return summarizeTurnStarted(payload, turn)
    case 'TOOL_CALL_STARTED':
      return summarizeToolCallStarted(payload)
    case 'TOOL_CALL_PROGRESS':
      return summarizeToolCallProgress(payload)
    case 'TOOL_CALL_FINISHED':
      return summarizeToolCallFinished(payload)
    case 'WAITING_FOR_CONFIRMATION':
      return summarizeWaitingForConfirmation(payload)
    case 'ASKUSER_REQUESTED':
      return { status: 'running', summary: 'askuser · waiting user' }
    case 'TURN_COMPLETED':
      return summarizeTurnCompleted(payload, turn)
    case 'TURN_FAILED':
      return summarizeTurnFailed(payload, turn)
    case 'TURN_CANCELLED':
      return { status: 'cancelled', summary: `turn ${turn} · cancelled` }
    default:
      return null
  }
}

export function dispatchWorkerAgentEvent(envelope: AgentEventEnvelope) {
  const workerId = String(envelope.source.worker_id ?? '').trim()
  const missionId = String(envelope.source.mission_id ?? '').trim()
  if (!workerId || !missionId) {
    return
  }

  const update = summarizeWorkerAgentEvent(envelope)
  if (!update) {
    return
  }

  const base = getOrCreateMissionUiState(missionId)
  const previous = base.workerStatuses[workerId]
  const featureId = extractWorkerFeatureIdFromPayload(envelope.payload)
    ?? previous?.featureId
    ?? ''

  commitMissionUiState({
    ...base,
    workerStatuses: upsertMissionWorkerStatus(base.workerStatuses, workerId, {
      featureId,
      status: update.status,
      summary: update.summary,
      updatedAt: envelope.ts,
    }),
  })
}


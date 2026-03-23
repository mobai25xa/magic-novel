import type { AgentRuntimeEventContext } from '../agent-event-context'

const KNOWN_WORKER_TYPES = new Set([
  'context',
  'draft',
  'review',
  'knowledge',
  'orchestrator',
  'other',
])

function toMaybeString(value: unknown) {
  if (typeof value !== 'string') return undefined
  const trimmed = value.trim()
  return trimmed ? trimmed : undefined
}

function normalizeWorkerType(value: unknown) {
  const raw = toMaybeString(value)?.toLowerCase()
  return raw && KNOWN_WORKER_TYPES.has(raw) ? raw : 'other'
}

export function handleWorkerEvent(ctx: AgentRuntimeEventContext) {
  switch (ctx.envelope.type) {
    case 'WORKER_STARTED':
      handleWorkerStarted(ctx)
      return
    case 'WORKER_COMPLETED':
      handleWorkerCompleted(ctx)
      return
    default:
      return
  }
}

function buildMeta(payload: Record<string, unknown>) {
  const workerType = normalizeWorkerType(payload.worker_type)
  const workerSessionId = toMaybeString(payload.worker_session_id)
  const scopeRef = toMaybeString(payload.scope_ref)
  const targetRef = toMaybeString(payload.target_ref)

  return {
    worker_type: workerType,
    worker_session_id: workerSessionId,
    scope_ref: scopeRef,
    target_ref: targetRef,
  }
}

function handleWorkerStarted(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx

  const meta = buildMeta(payload)
  const callId = toMaybeString(payload.call_id) || meta.worker_session_id
  const summary = toMaybeString(payload.summary) || `worker ${meta.worker_type} started`

  store.pushTurnEvent(turn, {
    type: 'WORKER_STARTED',
    ts,
    callId,
    summary,
    meta,
  })
}

function handleWorkerCompleted(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx

  const meta = buildMeta(payload)
  const callId = toMaybeString(payload.call_id) || meta.worker_session_id
  const summary = toMaybeString(payload.summary) || `worker ${meta.worker_type} completed`

  store.pushTurnEvent(turn, {
    type: 'WORKER_COMPLETED',
    ts,
    callId,
    summary,
    meta,
  })
}


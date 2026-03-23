import { agentTurnCancelClient } from '@/platform/tauri/clients/agent-engine-client'

import { useAgentChatStore } from '../store'

import type { AgentRuntimeEventContext } from './agent-event-context'
import type { AgentEventEnvelope } from './types'
import { dispatchWorkerAgentEvent } from './worker-agent-dispatch'

import { handleAskUserEvent } from './handlers/askuser'
import { handleToolEvent } from './handlers/tool'
import { handleTurnEvent } from './handlers/turn'
import { handleTurnFailedEvent } from './handlers/turn-failed'
import { handleUsageAndCompactionEvent } from './handlers/usage-and-compaction'
import { handleWorkerEvent } from './handlers/worker'

function extractClientRequestId(envelope: AgentEventEnvelope) {
  if (typeof envelope.client_request_id === 'string' && envelope.client_request_id.trim()) {
    return envelope.client_request_id
  }

  if (typeof envelope.payload.client_request_id === 'string' && envelope.payload.client_request_id.trim()) {
    return envelope.payload.client_request_id
  }

  return undefined
}

function getOldestPendingClientRequestId() {
  const pendingRequests = Object.values(useAgentChatStore.getState().pendingRequestsByClientRequestId)
  if (pendingRequests.length === 0) {
    return undefined
  }

  return [...pendingRequests]
    .sort((left, right) => left.createdAt - right.createdAt)
    .at(0)?.clientRequestId
}

function bindPendingRequestFromEnvelope(envelope: AgentEventEnvelope) {
  const store = useAgentChatStore.getState()
  const clientRequestId = extractClientRequestId(envelope)
    ?? store.clientRequestIdByTurnId[envelope.turn_id]
    ?? getOldestPendingClientRequestId()

  if (!clientRequestId) {
    return
  }

  let result: ReturnType<typeof store.bindPendingTurnRequest> | null = null
  try {
    result = store.bindPendingTurnRequest({
      clientRequestId,
      turn: envelope.turn_id,
    })
  } catch (error) {
    console.warn('[agent-event] bind pending request failed:', error)
    return
  }

  if (result?.cancelRequested) {
    agentTurnCancelClient({
      session_id: envelope.session_id,
      turn_id: envelope.turn_id,
    }).catch((error) => {
      console.error('[agent-event] cancel-after-bind failed:', error)
    })
  }
}

function isWorkerAgentEnvelope(envelope: AgentEventEnvelope) {
  if (envelope.source?.kind !== 'worker') {
    return false
  }

  const workerId = typeof envelope.source.worker_id === 'string' ? envelope.source.worker_id.trim() : ''
  const missionId = typeof envelope.source.mission_id === 'string' ? envelope.source.mission_id.trim() : ''
  return Boolean(workerId && missionId)
}

export function dispatchAgentEvent(envelope: AgentEventEnvelope) {
  if (isWorkerAgentEnvelope(envelope)) {
    dispatchWorkerAgentEvent(envelope)
    return
  }

  const store = useAgentChatStore.getState()
  if (envelope.session_id !== store.session_id) {
    return
  }

  bindPendingRequestFromEnvelope(envelope)

  dispatchAgentEventToHandlers({
    envelope,
    store,
    turn: envelope.turn_id,
    ts: envelope.ts,
    payload: envelope.payload,
  })
}

function dispatchAgentEventToHandlers(ctx: AgentRuntimeEventContext) {
  switch (ctx.envelope.type) {
    case 'TURN_FAILED':
      handleTurnFailedEvent(ctx)
      return
    case 'WORKER_STARTED':
    case 'WORKER_COMPLETED':
      handleWorkerEvent(ctx)
      return
    case 'TURN_STARTED':
    case 'PLAN_STARTED':
    case 'STREAMING_STARTED':
    case 'ASSISTANT_TEXT_DELTA':
    case 'THINKING_TEXT_DELTA':
    case 'TURN_COMPLETED':
    case 'TURN_CANCELLED':
      handleTurnEvent(ctx)
      return
    case 'TOOL_CALL_STARTED':
    case 'TOOL_CALL_PROGRESS':
    case 'TOOL_CALL_FINISHED':
    case 'WAITING_FOR_CONFIRMATION':
      handleToolEvent(ctx)
      return
    case 'ASKUSER_REQUESTED':
    case 'ASKUSER_ANSWERED':
      handleAskUserEvent(ctx)
      return
    case 'USAGE_UPDATE':
    case 'COMPACTION_STARTED':
    case 'COMPACTION_FINISHED':
    case 'COMPACTION_FALLBACK':
      handleUsageAndCompactionEvent(ctx)
      return
    default:
      // Unknown event type — log for debugging, no-op in store
      console.warn('[agent-event] unknown event type:', ctx.envelope.type, ctx.envelope)
  }
}

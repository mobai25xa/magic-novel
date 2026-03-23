import { buildToolExposureSummary, extractToolExposureMeta, normalizeStopReason } from '../utils'

import type { AgentRuntimeEventContext } from '../agent-event-context'
import {
  enqueueTurnAnswerDelta,
  enqueueTurnThinkingDelta,
  flushBatchedRuntimeUpdates,
} from '../batched-runtime-updates'

export function handleTurnEvent(ctx: AgentRuntimeEventContext) {
  switch (ctx.envelope.type) {
    case 'TURN_STARTED':
      handleTurnStarted(ctx)
      break
    case 'PLAN_STARTED':
      handlePlanStarted(ctx)
      break
    case 'STREAMING_STARTED':
      handleStreamingStarted(ctx)
      break
    case 'ASSISTANT_TEXT_DELTA':
      handleAssistantTextDelta(ctx)
      break
    case 'THINKING_TEXT_DELTA':
      handleThinkingTextDelta(ctx)
      break
    case 'TURN_COMPLETED':
      handleTurnCompleted(ctx)
      break
    case 'TURN_CANCELLED':
      handleTurnCancelled(ctx)
      break
    default:
      break
  }
}

function handleTurnStarted(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx
  store.markTurnStarted(turn)
  store.setStateStatus('thinking')
  store.setSessionRuntimeCapability({
    runtimeState: 'running',
    canContinue: false,
    canResume: false,
    readonlyReason: undefined,
  })
  store.pushTurnEvent(turn, {
    type: 'TURN_STARTED',
    ts,
    meta: extractToolExposureMeta(payload),
    summary: `turn started · ${String(payload.model_provider || '')}/${String(payload.model || '')}`,
  })
}

function handlePlanStarted(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx
  store.setTurnPhase(turn, 'planning')
  store.pushTurnEvent(turn, {
    type: 'PLAN_STARTED',
    ts,
    meta: extractToolExposureMeta(payload),
    summary: buildToolExposureSummary(payload),
  })
}

function handleStreamingStarted(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts } = ctx
  store.setTurnPhase(turn, 'planning')
  store.pushTurnEvent(turn, {
    type: 'STREAMING_STARTED',
    ts,
    summary: 'streaming started',
  })
}

function handleAssistantTextDelta(ctx: AgentRuntimeEventContext) {
  const { turn, payload } = ctx
  const delta = String(payload.delta ?? '')
  if (delta) {
    enqueueTurnAnswerDelta({
      sessionId: ctx.envelope.session_id,
      turn,
      delta,
    })
  }
}

function handleThinkingTextDelta(ctx: AgentRuntimeEventContext) {
  const { turn, payload } = ctx
  const delta = String(payload.delta ?? '')
  if (delta) {
    enqueueTurnThinkingDelta({
      sessionId: ctx.envelope.session_id,
      turn,
      delta,
    })
  }
}

function handleTurnCompleted(ctx: AgentRuntimeEventContext) {
  flushBatchedRuntimeUpdates({ sessionId: ctx.envelope.session_id, turn: ctx.turn })

  const { store, turn, ts, payload } = ctx
  const rawStopReason = String(payload.stop_reason ?? 'success')

  if (rawStopReason === 'waiting_confirmation') {
    store.setStateStatus('waiting_confirmation')
    store.setSessionRuntimeCapability({
      runtimeState: 'suspended_confirmation',
      canContinue: false,
      canResume: true,
      readonlyReason: undefined,
    })
  } else if (rawStopReason === 'waiting_askuser') {
    store.setStateStatus('waiting_askuser')
    store.setSessionRuntimeCapability({
      runtimeState: 'suspended_askuser',
      canContinue: false,
      canResume: true,
      readonlyReason: undefined,
    })
  } else {
    store.setStateStatus('idle')
    store.setSessionRuntimeCapability({
      runtimeState: rawStopReason === 'cancel' ? 'cancelled' : 'ready',
      canContinue: true,
      canResume: false,
      readonlyReason: undefined,
    })
  }

  const phase = rawStopReason === 'waiting_confirmation' || rawStopReason === 'waiting_askuser'
    ? 'tool_running'
    : rawStopReason === 'cancel'
      ? 'cancelled'
      : 'completed'

  store.setTurnPhase(turn, phase, {
    stopReason: normalizeStopReason(payload.stop_reason),
    finishedAt: ts,
  })
  if (typeof payload.latency_ms === 'number') {
    store.setLastTurnLatency(payload.latency_ms)
  }
  store.pushTurnEvent(turn, {
    type: 'TURN_COMPLETED',
    ts,
    meta: extractToolExposureMeta(payload),
    summary: `turn completed · ${rawStopReason}`,
  })
  if (rawStopReason !== 'waiting_confirmation' && rawStopReason !== 'waiting_askuser') {
    store.commitTurnTimelineSnapshot(turn)
  }
}

function handleTurnCancelled(ctx: AgentRuntimeEventContext) {
  flushBatchedRuntimeUpdates({ sessionId: ctx.envelope.session_id, turn: ctx.turn })

  const { store, turn, ts } = ctx
  store.setStateStatus('idle')
  store.setSessionRuntimeCapability({
    runtimeState: 'cancelled',
    canContinue: true,
    canResume: false,
    readonlyReason: undefined,
  })
  store.setTurnPhase(turn, 'cancelled', {
    stopReason: 'cancel',
    finishedAt: ts,
  })
  store.pushTurnEvent(turn, {
    type: 'TURN_CANCELLED',
    ts,
    summary: 'turn cancelled',
  })
  store.commitTurnTimelineSnapshot(turn)
}

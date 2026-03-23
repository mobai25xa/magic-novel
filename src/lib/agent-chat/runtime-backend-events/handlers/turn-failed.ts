import { asRecord } from '../utils'
import { flushBatchedRuntimeUpdates } from '../batched-runtime-updates'

import type { AgentRuntimeEventContext } from '../agent-event-context'

export function handleTurnFailedEvent(ctx: AgentRuntimeEventContext) {
  flushBatchedRuntimeUpdates({ sessionId: ctx.envelope.session_id, turn: ctx.turn })

  const { store, turn, ts, payload } = ctx

  store.setStateStatus('idle')
  const errorMsg = String(payload.error_message ?? payload.error_code ?? 'unknown error')
  const errorCode = String(payload.error_code ?? 'E_LLM_UNKNOWN')
  const errorDetail = asRecord(payload.error_detail)

  if (errorCode === 'E_CANCELLED') {
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
    return
  }

  store.setSessionRuntimeCapability({
    runtimeState: 'failed',
    canContinue: true,
    canResume: false,
    readonlyReason: undefined,
  })

  store.setTurnPhase(turn, 'failed', {
    error: errorMsg,
    turnError: {
      code: errorCode,
      message: errorMsg,
      detail: errorDetail ? buildTurnErrorDetail(errorDetail) : undefined,
    },
    finishedAt: ts,
  })
  store.pushTurnEvent(turn, {
    type: 'TURN_FAILED',
    ts,
    summary: errorMsg,
    meta: { error_code: errorCode, error_detail: errorDetail },
  })
  store.commitTurnTimelineSnapshot(turn)
}

function buildTurnErrorDetail(errorDetail: Record<string, unknown>) {
  return {
    provider: typeof errorDetail.provider === 'string' ? errorDetail.provider : undefined,
    model: typeof errorDetail.model === 'string' ? errorDetail.model : undefined,
    retryable: typeof errorDetail.retryable === 'boolean' ? errorDetail.retryable : undefined,
    diagnostic: typeof errorDetail.diagnostic === 'string' ? errorDetail.diagnostic : undefined,
    http_status: typeof errorDetail.http_status === 'number' ? errorDetail.http_status : undefined,
    retry_after_ms: typeof errorDetail.retry_after_ms === 'number' ? errorDetail.retry_after_ms : undefined,
    category_hint: typeof errorDetail.category_hint === 'string' ? errorDetail.category_hint : undefined,
    tool_name: typeof errorDetail.tool_name === 'string' ? errorDetail.tool_name : undefined,
    schema_path: typeof errorDetail.schema_path === 'string' ? errorDetail.schema_path : undefined,
    policy_source: typeof errorDetail.policy_source === 'string' ? errorDetail.policy_source : undefined,
    capability_preset: typeof errorDetail.capability_preset === 'string' ? errorDetail.capability_preset : undefined,
    exposure_reason: typeof errorDetail.exposure_reason === 'string' ? errorDetail.exposure_reason : undefined,
    turn_failed_classification: typeof errorDetail.turn_failed_classification === 'string'
      ? errorDetail.turn_failed_classification
      : undefined,
    provider_schema_error: typeof errorDetail.provider_schema_error === 'boolean'
      ? errorDetail.provider_schema_error
      : undefined,
    provider_400_error: typeof errorDetail.provider_400_error === 'boolean'
      ? errorDetail.provider_400_error
      : undefined,
    missing_tool_escalation: typeof errorDetail.missing_tool_escalation === 'boolean'
      ? errorDetail.missing_tool_escalation
      : undefined,
    tool_call_count: typeof errorDetail.tool_call_count === 'number' ? errorDetail.tool_call_count : undefined,
    rounds_executed: typeof errorDetail.rounds_executed === 'number' ? errorDetail.rounds_executed : undefined,
    exposed_tools: Array.isArray(errorDetail.exposed_tools)
      ? errorDetail.exposed_tools.filter((value): value is string => typeof value === 'string')
      : undefined,
    skipped_tools: Array.isArray(errorDetail.skipped_tools)
      ? errorDetail.skipped_tools
        .map((value) => asRecord(value))
        .filter((value): value is Record<string, unknown> => Boolean(value))
        .map((value) => ({
          tool_name: typeof value.tool_name === 'string' ? value.tool_name : undefined,
          error: typeof value.error === 'string' ? value.error : undefined,
        }))
      : undefined,
  }
}

import type { AgentRuntimeEventContext } from '../agent-event-context'

export function handleUsageAndCompactionEvent(ctx: AgentRuntimeEventContext) {
  const { store, turn, ts, payload } = ctx

  switch (ctx.envelope.type) {
    case 'USAGE_UPDATE': {
      // Usage tracking — currently just push as timeline event for observability
      store.pushTurnEvent(turn, {
        type: 'TURN_STARTED', // reuse; no dedicated usage event type
        ts,
        meta: payload,
        summary: `tokens: in=${String(payload.input_tokens ?? 0)} out=${String(payload.output_tokens ?? 0)}`,
      })
      break
    }

    case 'COMPACTION_STARTED': {
      store.setStateStatus('compacting')
      store.setTurnPhase(turn, 'compacting')
      store.pushTurnEvent(turn, {
        type: 'COMPACTION_STARTED',
        ts,
        summary: `compaction started: ${String(payload.reason ?? '')}`,
      })
      break
    }

    case 'COMPACTION_FINISHED': {
      store.pushTurnEvent(turn, {
        type: 'COMPACTION_FINISHED',
        ts,
        summary: 'compaction finished',
        meta: payload.meta as Record<string, unknown> | undefined,
      })
      break
    }

    case 'COMPACTION_FALLBACK': {
      store.pushTurnEvent(turn, {
        type: 'COMPACTION_FALLBACK',
        ts,
        summary: String(payload.message ?? payload.reason ?? 'compaction fallback'),
        meta: payload,
      })
      break
    }

    default:
      break
  }
}


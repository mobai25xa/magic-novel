import type { AgentUiTimelineEvent } from './types'
import type { AgentChatStateSlice, PushTurnEventInput } from './store-helpers'
import { ensureTurnState, pushWithLimit } from './store-helpers'

const MAX_TURN_EVENTS = 60

function buildEventId(type: AgentUiTimelineEvent['type']) {
  return `evt_${type.toLowerCase()}_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
}

export function reducePushTurnEvent(
  state: AgentChatStateSlice,
  turn: number,
  event: PushTurnEventInput,
): Partial<AgentChatStateSlice> {
  const ts = event.ts ?? Date.now()
  const currentPhase = state.turnById[turn]?.phase
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, currentPhase ?? 'planning', ts)
  const existing = state.eventsByTurnId[turn] || []
  const lastSeq = existing.at(-1)?.seq ?? 0
  const seq = typeof event.seq === 'number' && Number.isFinite(event.seq) && event.seq > lastSeq
    ? Math.floor(event.seq)
    : lastSeq + 1
  const nextEvent: AgentUiTimelineEvent = {
    id: event.id || buildEventId(event.type),
    turn,
    type: event.type,
    ts,
    seq,
    summary: event.summary,
    callId: event.callId,
    delta: event.delta,
    meta: event.meta,
  }

  return {
    eventsByTurnId: {
      ...state.eventsByTurnId,
      [turn]: pushWithLimit(existing, nextEvent, MAX_TURN_EVENTS),
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}


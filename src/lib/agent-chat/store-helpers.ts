import type { AgentLoopStopReason, AgentStateStatus } from '@/agent/types'
import type { TurnTimelineSnapshot } from './timeline'

import type {
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnError,
  AgentUiTurnPhase,
  AgentUiTurnState,
  ChatUiMessage,
} from './types'

export interface AgentChatStateSlice {
  turn: number
  messages: ChatUiMessage[]
  stateStatus: AgentStateStatus
  turnOrder: number[]
  turnById: Record<number, AgentUiTurnState>
  answerByTurnId: Record<number, string>
  thinkingByTurnId: Record<number, string>
  stepsByTurnId: Record<number, AgentUiToolStep[]>
  eventsByTurnId: Record<number, AgentUiTimelineEvent[]>
  committedTimelineByTurnId: Record<number, TurnTimelineSnapshot>
}

export interface SetTurnPhaseOptions {
  stopReason?: AgentLoopStopReason
  error?: string
  turnError?: AgentUiTurnError
  finishedAt?: number
}

export type PushTurnEventInput = Omit<AgentUiTimelineEvent, 'id' | 'turn' | 'ts' | 'seq'> & {
  id?: string
  ts?: number
  seq?: number
}

export function pushWithLimit<T>(items: T[], item: T, max: number): T[] {
  const next = [...items, item]
  if (next.length <= max) return next
  return next.slice(next.length - max)
}

export function withLimit<T>(items: T[], max: number): T[] {
  if (items.length <= max) return items
  return items.slice(items.length - max)
}

export function ensureTurnState(
  turnById: Record<number, AgentUiTurnState>,
  turnOrder: number[],
  turn: number,
  phase: AgentUiTurnPhase,
  ts: number,
): { turnById: Record<number, AgentUiTurnState>; turnOrder: number[] } {
  const existing = turnById[turn]
  if (existing) {
    return {
      turnById: {
        ...turnById,
        [turn]: {
          ...existing,
          phase,
          updatedAt: ts,
        },
      },
      turnOrder,
    }
  }

  return {
    turnById: {
      ...turnById,
      [turn]: {
        turn,
        phase,
        startedAt: ts,
        updatedAt: ts,
      },
    },
    turnOrder: turnOrder.includes(turn) ? turnOrder : [...turnOrder, turn],
  }
}

export function mapRuntimeStatusToTurnPhase(
  status: AgentStateStatus,
  hasToolSteps: boolean,
  previousPhase?: AgentUiTurnPhase,
): AgentUiTurnPhase {
  if (status === 'compacting') return 'compacting'
  if (status === 'executing_tool' || status === 'waiting_confirmation' || status === 'waiting_askuser') return 'tool_running'
  if (status === 'thinking') return hasToolSteps ? 'synthesizing' : 'planning'
  if (status === 'idle') {
    if (previousPhase === 'failed' || previousPhase === 'cancelled') {
      return previousPhase
    }
    return 'completed'
  }
  return hasToolSteps ? 'synthesizing' : 'planning'
}

export function reduceAddUiMessage(state: AgentChatStateSlice, message: ChatUiMessage) {
  const messages = pushWithLimit(state.messages, message, 300)

  if (message.role !== 'assistant' || !Number.isFinite(message.turn) || message.turn <= 0) {
    return {
      messages,
    }
  }

  const turn = Math.floor(message.turn)
  const ts = typeof message.ts === 'number' && Number.isFinite(message.ts)
    ? message.ts
    : Date.now()
  const phase = state.turnById[turn]?.phase ?? 'synthesizing'
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, phase, ts)

  return {
    messages,
    answerByTurnId: {
      ...state.answerByTurnId,
      [turn]: message.content,
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

export function reduceSetStateStatus(
  state: AgentChatStateSlice,
  status: AgentStateStatus,
): Partial<AgentChatStateSlice> {
  const turn = state.turn
  if (!turn || !state.turnById[turn]) {
    return { stateStatus: status }
  }

  const phase = mapRuntimeStatusToTurnPhase(
    status,
    (state.stepsByTurnId[turn]?.length ?? 0) > 0,
    state.turnById[turn]?.phase,
  )
  const ts = Date.now()

  return {
    stateStatus: status,
    turnById: {
      ...state.turnById,
      [turn]: {
        ...state.turnById[turn],
        phase,
        updatedAt: ts,
        ...(phase === 'completed' ? { finishedAt: ts } : {}),
      },
    },
  }
}

export function reduceMarkTurnStarted(
  state: AgentChatStateSlice,
  turn: number,
): Partial<AgentChatStateSlice> {
  const ts = Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'queued', ts)

  return {
    turnById: ensured.turnById,
    turnOrder: ensured.turnOrder,
    answerByTurnId: state.answerByTurnId[turn]
      ? state.answerByTurnId
      : { ...state.answerByTurnId, [turn]: '' },
    thinkingByTurnId: state.thinkingByTurnId[turn]
      ? state.thinkingByTurnId
      : { ...state.thinkingByTurnId, [turn]: '' },
    stepsByTurnId: state.stepsByTurnId[turn]
      ? state.stepsByTurnId
      : { ...state.stepsByTurnId, [turn]: [] },
    eventsByTurnId: state.eventsByTurnId[turn]
      ? state.eventsByTurnId
      : { ...state.eventsByTurnId, [turn]: [] },
  }
}

export function reduceSetTurnPhase(
  state: AgentChatStateSlice,
  turn: number,
  phase: AgentUiTurnPhase,
  options?: SetTurnPhaseOptions,
): Partial<AgentChatStateSlice> {
  const ts = options?.finishedAt ?? Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, phase, ts)
  const prev = ensured.turnById[turn]

  return {
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...prev,
        phase,
        stopReason: options?.stopReason ?? prev.stopReason,
        error: options?.error ?? prev.error,
        turnError: options?.turnError ?? prev.turnError,
        updatedAt: ts,
        ...(phase === 'completed' || phase === 'cancelled' || phase === 'failed'
          ? { finishedAt: options?.finishedAt ?? ts }
          : {}),
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

export function reduceAppendTurnAnswerDelta(
  state: AgentChatStateSlice,
  turn: number,
  delta: string,
): Partial<AgentChatStateSlice> {
  const ts = Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'synthesizing', ts)

  return {
    answerByTurnId: {
      ...state.answerByTurnId,
      [turn]: `${state.answerByTurnId[turn] || ''}${delta}`,
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase: 'synthesizing',
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

export function reduceAppendTurnThinkingDelta(
  state: AgentChatStateSlice,
  turn: number,
  delta: string,
): Partial<AgentChatStateSlice> {
  const ts = Date.now()
  const previousPhase = state.turnById[turn]?.phase
  const hasToolSteps = (state.stepsByTurnId[turn]?.length ?? 0) > 0
  const phase = previousPhase === 'tool_running' || previousPhase === 'compacting'
    ? previousPhase
    : hasToolSteps
      ? 'synthesizing'
      : 'planning'
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, phase, ts)

  return {
    thinkingByTurnId: {
      ...state.thinkingByTurnId,
      [turn]: `${state.thinkingByTurnId[turn] || ''}${delta}`,
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase,
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

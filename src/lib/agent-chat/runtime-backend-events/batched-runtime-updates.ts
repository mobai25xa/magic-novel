import { useAgentChatStore } from '../store'
import {
  reduceAppendTurnAnswerDelta,
  reduceAppendTurnThinkingDelta,
} from '../store-helpers'
import type { AgentChatState } from '../store-types'
import {
  reduceUpsertToolStepProgress,
  type ToolStepProgressInput,
} from '../store-helpers-tool-steps'

const FLUSH_INTERVAL_MS = 200

type PendingToolProgress = ToolStepProgressInput

type PendingRuntimeUpdates = {
  answerDeltaByTurn: Map<number, string>
  thinkingDeltaByTurn: Map<number, string>
  toolProgressByTurnAndCallId: Map<number, Map<string, PendingToolProgress>>
}

const pendingBySessionId = new Map<string, PendingRuntimeUpdates>()
let flushTimer: ReturnType<typeof setTimeout> | undefined

function getOrCreatePending(sessionId: string): PendingRuntimeUpdates {
  const existing = pendingBySessionId.get(sessionId)
  if (existing) {
    return existing
  }

  const next: PendingRuntimeUpdates = {
    answerDeltaByTurn: new Map(),
    thinkingDeltaByTurn: new Map(),
    toolProgressByTurnAndCallId: new Map(),
  }
  pendingBySessionId.set(sessionId, next)
  return next
}

function hasAnyPending(pending: PendingRuntimeUpdates) {
  return pending.answerDeltaByTurn.size > 0
    || pending.thinkingDeltaByTurn.size > 0
    || pending.toolProgressByTurnAndCallId.size > 0
}

function scheduleFlush() {
  if (flushTimer) return
  flushTimer = setTimeout(() => {
    flushTimer = undefined
    flushBatchedRuntimeUpdates()
  }, FLUSH_INTERVAL_MS)
}

function dropIfSessionMismatch(sessionId: string) {
  const store = useAgentChatStore.getState()
  if (store.session_id !== sessionId) {
    pendingBySessionId.delete(sessionId)
    return true
  }
  return false
}

export function enqueueTurnAnswerDelta(input: { sessionId: string; turn: number; delta: string }) {
  if (!input.delta) return
  if (dropIfSessionMismatch(input.sessionId)) return

  const store = useAgentChatStore.getState()
  const phase = store.turnById[input.turn]?.phase
  if (phase === 'completed' || phase === 'cancelled' || phase === 'failed') {
    return
  }

  const pending = getOrCreatePending(input.sessionId)
  const existing = pending.answerDeltaByTurn.get(input.turn) || ''
  pending.answerDeltaByTurn.set(input.turn, `${existing}${input.delta}`)
  scheduleFlush()
}

export function enqueueTurnThinkingDelta(input: { sessionId: string; turn: number; delta: string }) {
  if (!input.delta) return
  if (dropIfSessionMismatch(input.sessionId)) return

  const store = useAgentChatStore.getState()
  const phase = store.turnById[input.turn]?.phase
  if (phase === 'completed' || phase === 'cancelled' || phase === 'failed') {
    return
  }

  const pending = getOrCreatePending(input.sessionId)
  const existing = pending.thinkingDeltaByTurn.get(input.turn) || ''
  pending.thinkingDeltaByTurn.set(input.turn, `${existing}${input.delta}`)
  scheduleFlush()
}

export function enqueueToolStepProgress(input: {
  sessionId: string
  turn: number
  progress: PendingToolProgress
}) {
  if (dropIfSessionMismatch(input.sessionId)) return

  const store = useAgentChatStore.getState()
  const turnPhase = store.turnById[input.turn]?.phase
  if (turnPhase === 'completed' || turnPhase === 'cancelled' || turnPhase === 'failed') {
    return
  }

  const existingStep = store.stepsByTurnId[input.turn]?.find((step) => step.callId === input.progress.callId)
  if (existingStep?.status === 'success' || existingStep?.status === 'error' || existingStep?.status === 'cancelled') {
    return
  }

  const pending = getOrCreatePending(input.sessionId)
  const byCallId = pending.toolProgressByTurnAndCallId.get(input.turn) || new Map<string, PendingToolProgress>()
  byCallId.set(input.progress.callId, input.progress)
  pending.toolProgressByTurnAndCallId.set(input.turn, byCallId)
  scheduleFlush()
}

export function flushBatchedRuntimeUpdates(input?: { sessionId?: string; turn?: number }) {
  const store = useAgentChatStore.getState()
  const sessionId = input?.sessionId ?? store.session_id
  const pending = pendingBySessionId.get(sessionId)
  if (!pending) return
  if (dropIfSessionMismatch(sessionId)) return

  const turnsToFlush = typeof input?.turn === 'number'
    ? [input.turn]
    : [...new Set([
      ...pending.answerDeltaByTurn.keys(),
      ...pending.thinkingDeltaByTurn.keys(),
      ...pending.toolProgressByTurnAndCallId.keys(),
    ])].sort((a, b) => a - b)

  const answerDeltas: Array<[number, string]> = []
  const thinkingDeltas: Array<[number, string]> = []
  const toolProgress: Array<[number, PendingToolProgress]> = []

  for (const turn of turnsToFlush) {
    const answerDelta = pending.answerDeltaByTurn.get(turn)
    if (answerDelta) {
      answerDeltas.push([turn, answerDelta])
    }

    const thinkingDelta = pending.thinkingDeltaByTurn.get(turn)
    if (thinkingDelta) {
      thinkingDeltas.push([turn, thinkingDelta])
    }

    const byCallId = pending.toolProgressByTurnAndCallId.get(turn)
    if (byCallId && byCallId.size > 0) {
      for (const update of byCallId.values()) {
        toolProgress.push([turn, update])
      }
    }
  }

  if (answerDeltas.length === 0 && thinkingDeltas.length === 0 && toolProgress.length === 0) {
    return
  }

  useAgentChatStore.setState((state) => {
    if (state.session_id !== sessionId) {
      return {}
    }

    let working = state as unknown as AgentChatState
    let patch: Partial<AgentChatState> = {}

    const apply = (next: Partial<AgentChatState>) => {
      working = {
        ...working,
        ...next,
      } as AgentChatState
      patch = {
        ...patch,
        ...next,
      }
    }

    for (const [turn, delta] of answerDeltas) {
      apply(reduceAppendTurnAnswerDelta(working, turn, delta) as Partial<AgentChatState>)
    }

    for (const [turn, delta] of thinkingDeltas) {
      apply(reduceAppendTurnThinkingDelta(working, turn, delta) as Partial<AgentChatState>)
    }

    for (const [turn, update] of toolProgress) {
      apply(reduceUpsertToolStepProgress(working, turn, update) as Partial<AgentChatState>)
    }

    return patch
  })

  for (const turn of turnsToFlush) {
    pending.answerDeltaByTurn.delete(turn)
    pending.thinkingDeltaByTurn.delete(turn)
    pending.toolProgressByTurnAndCallId.delete(turn)
  }

  if (!hasAnyPending(pending)) {
    pendingBySessionId.delete(sessionId)
  }
}

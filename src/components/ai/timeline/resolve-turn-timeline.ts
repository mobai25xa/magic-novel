import { buildTurnTimelineSnapshot, type TurnTimelineSnapshot } from '@/lib/agent-chat/timeline'

import type {
  ResolvedTurnTimeline,
  ResolveTurnTimelineInput,
} from './timeline-types'

function isTerminalPhase(phase: ResolveTurnTimelineInput['phase']) {
  return phase === 'completed' || phase === 'cancelled' || phase === 'failed'
}

function normalizeSnapshot(snapshot: unknown): TurnTimelineSnapshot | undefined {
  if (!snapshot || typeof snapshot !== 'object') {
    return undefined
  }

  const value = snapshot as Partial<TurnTimelineSnapshot>
  if (value.version !== 2 || typeof value.turn !== 'number' || !Array.isArray(value.blocks)) {
    return undefined
  }

  return {
    version: 2,
    turn: value.turn,
    blocks: value.blocks,
    stage: value.stage === 'response' || value.stage === 'thinking' || value.stage === 'streaming'
      ? value.stage
      : undefined,
    createdAt: typeof value.createdAt === 'number' && Number.isFinite(value.createdAt)
      ? value.createdAt
      : Date.now(),
  }
}

export function resolveTurnTimeline(input: ResolveTurnTimelineInput): ResolvedTurnTimeline {
  const snapshot = normalizeSnapshot(input.snapshot)
  if (snapshot && snapshot.turn === input.turn && isTerminalPhase(input.phase)) {
    return {
      blocks: snapshot.blocks,
      stage: snapshot.stage ?? 'response',
    }
  }

  const toolStepsByCallId = Object.fromEntries(
    input.toolSteps.map((step) => [step.callId, step]),
  )

  const built = buildTurnTimelineSnapshot({
    turn: input.turn,
    events: input.events,
    toolStepsByCallId,
    answerText: input.answerText,
    thinkingText: input.thinkingText,
    running: input.running,
    phase: input.phase,
  })

  return {
    blocks: built.blocks,
    stage: built.stage ?? 'response',
  }
}

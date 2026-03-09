import type {
  AgentSessionEvent,
  AgentSessionHydrationAuthority,
  AgentSessionHydrationStatus,
  AgentSessionMeta,
  AgentSessionReadonlyReason,
  AgentSessionRuntimeState,
} from './session-types'

type SessionHydrationShape = {
  sessionId: string
  hydrationStatus: AgentSessionHydrationStatus
  runtimeState: AgentSessionRuntimeState
  canContinue: boolean
  canResume: boolean
  readonlyReason?: AgentSessionReadonlyReason
  warnings: string[]
} & AgentSessionHydrationAuthority

function toNonNegativeInteger(value: unknown) {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) {
    return undefined
  }

  return Math.floor(value)
}

function toPositiveInteger(value: unknown) {
  const normalized = toNonNegativeInteger(value)
  if (!normalized || normalized < 1) {
    return undefined
  }

  return normalized
}

function toOptionalString(value: unknown) {
  if (typeof value !== 'string') {
    return undefined
  }

  const trimmed = value.trim()
  return trimmed ? trimmed : undefined
}

export function inferHistoricalLastTurn(input: {
  events?: AgentSessionEvent[]
  meta?: AgentSessionMeta
  replayTurn?: number
}) {
  let maxTurn = toNonNegativeInteger(input.meta?.last_turn) ?? 0

  const replayTurn = toNonNegativeInteger(input.replayTurn)
  if (typeof replayTurn === 'number' && replayTurn > maxTurn) {
    maxTurn = replayTurn
  }

  for (const event of input.events || []) {
    const turn = toNonNegativeInteger(event.turn)
    if (typeof turn === 'number' && turn > maxTurn) {
      maxTurn = turn
    }
  }

  return maxTurn
}

export function normalizeSessionHydration<T extends SessionHydrationShape>(
  input: T,
): T & AgentSessionHydrationAuthority {
  const lastTurn = toNonNegativeInteger(input.lastTurn)
  const derivedNextTurnId = typeof lastTurn === 'number' ? lastTurn + 1 : undefined
  const explicitNextTurnId = toPositiveInteger(input.nextTurnId)
  const baselineNextTurnId = explicitNextTurnId && derivedNextTurnId
    ? Math.max(explicitNextTurnId, derivedNextTurnId)
    : explicitNextTurnId ?? derivedNextTurnId
  const nextTurnId = input.readonlyReason
    ? undefined
    : baselineNextTurnId ?? (input.canContinue || input.canResume ? 1 : undefined)
  const sessionRevision = toNonNegativeInteger(input.sessionRevision)
  const hydrationSource = toOptionalString(input.hydrationSource) || input.hydrationStatus

  return {
    ...input,
    lastTurn,
    nextTurnId,
    sessionRevision,
    hydrationSource,
    warnings: [...input.warnings],
  }
}

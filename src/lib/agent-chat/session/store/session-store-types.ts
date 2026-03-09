import type {
  AgentSessionEvent,
  AgentSessionHydrationAuthority,
  AgentSessionHydrationStatus,
  AgentSessionMeta,
  AgentSessionReadonlyReason,
  AgentSessionRuntimeState,
} from '../session-types'

import type { SessionPersistenceStorePatch } from './session-store-contract'

export type SetStoreState = (next: Partial<SessionPersistenceStorePatch>) => void
export type GetStoreState = () => SessionPersistenceStorePatch

export type ApplySessionEvents = (input: {
  sessionId: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
  replayedAt?: number
}) => void

export type ApplySessionHydration = (input: {
  sessionId: string
  hydrationStatus: AgentSessionHydrationStatus
  runtimeState: AgentSessionRuntimeState
  canContinue: boolean
  canResume: boolean
  readonlyReason?: AgentSessionReadonlyReason
  warnings: string[]
} & AgentSessionHydrationAuthority) => void

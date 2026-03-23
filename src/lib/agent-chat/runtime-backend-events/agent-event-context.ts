import type { AgentChatStoreState } from './agent-chat-store-state'
import type { AgentEventEnvelope } from './types'

export type AgentRuntimeEventContext = {
  envelope: AgentEventEnvelope
  store: AgentChatStoreState
  turn: number
  ts: number
  payload: Record<string, unknown>
}


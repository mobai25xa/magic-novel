import type {
  AgentTurnStartOutput,
  ApprovalMode,
  CapabilityMode,
  ClarificationMode,
} from './agent-engine-client'
import { invokeTauri } from './core'

export interface InspirationTurnStartInput {
  session_id: string
  client_request_id?: string
  user_text: string
  model?: string
  provider?: string
  base_url?: string
  api_key?: string
  system_prompt?: string
  capability_mode?: CapabilityMode
  approval_mode?: ApprovalMode
  clarification_mode?: ClarificationMode
}

export interface InspirationTurnCancelInput {
  session_id: string
  turn_id: number
}

export async function inspirationTurnStartClient(
  input: InspirationTurnStartInput,
): Promise<AgentTurnStartOutput> {
  return invokeTauri<AgentTurnStartOutput>('inspiration_turn_start', { input })
}

export async function inspirationTurnCancelClient(
  input: InspirationTurnCancelInput,
): Promise<void> {
  return invokeTauri<void>('inspiration_turn_cancel', { input })
}

import { invokeTauri } from './core'

// ── Input types (mirror Rust DTOs) ──────────────────────────────

export type ApprovalMode = 'confirm_writes' | 'auto'
export type CapabilityMode = 'writing' | 'planning'
export type ClarificationMode = 'interactive' | 'headless_defer'

export interface AgentTurnStartInput {
  session_id: string
  client_request_id: string
  user_text: string
  project_path: string
  model?: string
  provider?: string
  base_url?: string
  api_key?: string
  system_prompt?: string
  active_chapter_path?: string
  capability_mode?: CapabilityMode
  approval_mode?: ApprovalMode
  clarification_mode?: ClarificationMode
}

export interface AgentTurnStartOutput {
  session_id: string
  turn_id: number
  client_request_id?: string
  session_revision?: number
  hydration_status?: string
}

export interface AgentTurnCancelInput {
  session_id: string
  turn_id: number
}

export interface AgentTurnResumeInput {
  session_id: string
  turn_id: number
  resume_input: ResumeInputConfirmation | ResumeInputAskUser
}

export interface ResumeInputConfirmation {
  kind: 'confirmation'
  allowed: boolean
}

export interface ResumeInputAskUser {
  kind: 'askuser'
  answers: unknown
}

// ── Tauri invoke wrappers ───────────────────────────────────────

export async function agentTurnStartClient(
  input: AgentTurnStartInput,
): Promise<AgentTurnStartOutput> {
  return invokeTauri<AgentTurnStartOutput>('agent_turn_start', { input })
}

export async function agentTurnCancelClient(
  input: AgentTurnCancelInput,
): Promise<void> {
  return invokeTauri<void>('agent_turn_cancel', { input })
}

export async function agentTurnResumeClient(
  input: AgentTurnResumeInput,
): Promise<void> {
  return invokeTauri<void>('agent_turn_resume', { input })
}

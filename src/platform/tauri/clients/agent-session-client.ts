import type {
  AgentSessionEvent,
  AgentSessionHydrationStatus,
  AgentSessionMeta,
  AgentSessionReadonlyReason,
  AgentSessionRuntimeState,
} from '@/lib/agent-chat/session/session-types'

export interface AgentSessionCreateInput {
  projectPath: string
  title?: string
  activeChapterPath?: string
}

export interface AgentSessionCreateOutput {
  schema_version: number
  session_id: string
  created_at: number
}

export interface AgentSessionAppendEventsInput {
  projectPath: string
  sessionId: string
  events: AgentSessionEvent[]
}

export interface AgentSessionLoadInput {
  projectPath: string
  sessionId: string
}

export interface AgentSessionLoadOutput {
  schema_version: number
  session_id: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
}

export interface AgentSessionHydrateInput {
  projectPath: string
  sessionId: string
}

export interface AgentSessionHydrateOutput {
  schema_version: number
  session_id: string
  hydration_status: AgentSessionHydrationStatus
  runtime_state: AgentSessionRuntimeState
  can_continue: boolean
  can_resume: boolean
  readonly_reason?: AgentSessionReadonlyReason
  warnings: string[]
  last_turn?: number
  next_turn_id?: number
  session_revision?: number
  hydration_source?: string
}

export interface AgentSessionListInput {
  projectPath: string
  limit?: number
}

export interface AgentSessionUpdateMetaInput {
  projectPath: string
  sessionId: string
  title?: string
  activeChapterPath?: string
}

export interface AgentSessionRecoverInput {
  projectPath: string
  sessionId?: string
}

export interface AgentSessionDeleteInput {
  projectPath: string
  sessionId: string
}

export interface AgentSessionRecoverOutput {
  schema_version: number
  repaired_files: number
  truncated_bytes: number
  notes: string[]
  quarantined_sessions?: string[]
  manual_repair_actions?: string[]
}

import { invokeTauri } from './core'

export async function agentSessionCreateClient(
  input: AgentSessionCreateInput,
): Promise<AgentSessionCreateOutput> {
  return invokeTauri('agent_session_create', {
    input: {
      project_path: input.projectPath,
      title: input.title,
      active_chapter_path: input.activeChapterPath,
    },
  })
}

export async function agentSessionAppendEventsClient(input: AgentSessionAppendEventsInput): Promise<void> {
  return invokeTauri('agent_session_append_events', {
    input: {
      project_path: input.projectPath,
      session_id: input.sessionId,
      events: input.events,
    },
  })
}

export async function agentSessionLoadClient(input: AgentSessionLoadInput): Promise<AgentSessionLoadOutput> {
  return invokeTauri('agent_session_load', {
    input: {
      project_path: input.projectPath,
      session_id: input.sessionId,
    },
  })
}

export async function agentSessionHydrateClient(input: AgentSessionHydrateInput): Promise<AgentSessionHydrateOutput> {
  return invokeTauri('agent_session_hydrate', {
    input: {
      project_path: input.projectPath,
      session_id: input.sessionId,
    },
  })
}

export async function agentSessionListClient(input: AgentSessionListInput): Promise<AgentSessionMeta[]> {
  return invokeTauri('agent_session_list', {
    input: {
      project_path: input.projectPath,
      limit: input.limit,
    },
  })
}

export async function agentSessionUpdateMetaClient(input: AgentSessionUpdateMetaInput): Promise<void> {
  return invokeTauri('agent_session_update_meta', {
    input: {
      project_path: input.projectPath,
      session_id: input.sessionId,
      title: input.title,
      active_chapter_path: input.activeChapterPath,
    },
  })
}

export async function agentSessionRecoverClient(
  input: AgentSessionRecoverInput,
): Promise<AgentSessionRecoverOutput> {
  return invokeTauri('agent_session_recover', {
    input: {
      project_path: input.projectPath,
      session_id: input.sessionId,
    },
  })
}

export async function agentSessionDeleteClient(input: AgentSessionDeleteInput): Promise<void> {
  return invokeTauri('agent_session_delete', {
    input: {
      project_path: input.projectPath,
      session_id: input.sessionId,
    },
  })
}

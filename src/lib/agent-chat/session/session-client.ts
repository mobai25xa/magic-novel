import {
  agentSessionAppendEvents,
  agentSessionCreate,
  agentSessionDelete,
  agentSessionHydrate,
  agentSessionList,
  agentSessionLoad,
  agentSessionUpdateMeta,
} from '@/lib/tauri-commands'
import { logUiMetric } from '@/agent/telemetry'

import {
  AGENT_SESSION_SCHEMA_VERSION,
  type AgentSessionHydrationAuthority,
  type AgentSessionEvent,
  type AgentSessionHydrationStatus,
  type AgentSessionMeta,
  type AgentSessionReadonlyReason,
  type AgentSessionRuntimeState,
} from './session-types'
import { normalizeSessionHydration } from './session-hydration'

export interface CreatePersistedSessionInput {
  projectPath: string
  title?: string
  activeChapterPath?: string
}

export interface LoadPersistedSessionInput {
  projectPath: string
  sessionId: string
}

export interface ListPersistedSessionInput {
  projectPath: string
  limit?: number
}

export interface AppendPersistedSessionEventsInput {
  projectPath: string
  sessionId: string
  events: AgentSessionEvent[]
}

export interface DeletePersistedSessionInput {
  projectPath: string
  sessionId: string
}

export interface RenamePersistedSessionInput {
  projectPath: string
  sessionId: string
  title: string
}

export interface LoadPersistedSessionOutput {
  sessionId: string
  events: AgentSessionEvent[]
  meta?: AgentSessionMeta
}

export interface HydratePersistedSessionInput {
  projectPath: string
  sessionId: string
}

interface HydratePersistedSessionOutputBase {
  schemaVersion: number
  sessionId: string
  hydrationStatus: AgentSessionHydrationStatus
  runtimeState: AgentSessionRuntimeState
  canContinue: boolean
  canResume: boolean
  readonlyReason?: AgentSessionReadonlyReason
  warnings: string[]
}

export type HydratePersistedSessionOutput = HydratePersistedSessionOutputBase & AgentSessionHydrationAuthority

export async function createPersistedSessionClient(input: CreatePersistedSessionInput): Promise<AgentSessionMeta> {
  const created = await agentSessionCreate({
    projectPath: input.projectPath,
    title: input.title,
    activeChapterPath: input.activeChapterPath,
  })

  return {
    schema_version: AGENT_SESSION_SCHEMA_VERSION,
    session_id: created.session_id,
    created_at: created.created_at,
    updated_at: created.created_at,
    title: input.title,
    active_chapter_path: input.activeChapterPath,
    compaction_count: 0,
  }
}

export async function listPersistedSessionsClient(input: ListPersistedSessionInput): Promise<AgentSessionMeta[]> {
  return agentSessionList({
    projectPath: input.projectPath,
    limit: input.limit,
  })
}

export async function loadPersistedSessionClient(input: LoadPersistedSessionInput): Promise<LoadPersistedSessionOutput> {
  const loaded = await agentSessionLoad({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })

  return {
    sessionId: loaded.session_id,
    events: loaded.events,
    meta: loaded.meta,
  }
}

export async function hydratePersistedSessionClient(
  input: HydratePersistedSessionInput,
): Promise<HydratePersistedSessionOutput> {
  const hydrated = await agentSessionHydrate({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })

  return normalizeSessionHydration({
    schemaVersion: hydrated.schema_version,
    sessionId: hydrated.session_id,
    hydrationStatus: hydrated.hydration_status,
    runtimeState: hydrated.runtime_state,
    canContinue: hydrated.can_continue,
    canResume: hydrated.can_resume,
    readonlyReason: hydrated.readonly_reason,
    warnings: hydrated.warnings,
    lastTurn: hydrated.last_turn,
    nextTurnId: hydrated.next_turn_id,
    sessionRevision: hydrated.session_revision,
    hydrationSource: hydrated.hydration_source,
  })
}

export async function appendPersistedSessionEventsClient(input: AppendPersistedSessionEventsInput): Promise<void> {
  if (input.events.length === 0) {
    return
  }

  try {
    await agentSessionAppendEvents({
      projectPath: input.projectPath,
      sessionId: input.sessionId,
      events: input.events,
    })

    logUiMetric({
      sessionId: input.sessionId,
      metric: 'agent_session_append_events_success_count',
      value: 1,
      tags: {
        project_path: input.projectPath,
        event_count: input.events.length,
      },
    })
  } catch (error) {
    logUiMetric({
      sessionId: input.sessionId,
      metric: 'agent_session_append_events_error_count',
      value: 1,
      tags: {
        project_path: input.projectPath,
        event_count: input.events.length,
      },
    })
    throw error
  }
}

export async function deletePersistedSessionClient(input: DeletePersistedSessionInput): Promise<void> {
  await agentSessionDelete({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  })
}

export async function renamePersistedSessionClient(input: RenamePersistedSessionInput): Promise<void> {
  await agentSessionUpdateMeta({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    title: input.title,
  })
}

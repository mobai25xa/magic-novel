export const AGENT_SESSION_SCHEMA_VERSION = 1 as const

export const AGENT_SESSION_STORAGE = {
  segments: ['magic_novel', 'ai', 'sessions'] as const,
  indexFile: 'index.json' as const,
  settingsSuffix: '.settings.json' as const,
  streamSuffix: '.jsonl' as const,
}

export type AgentSessionEventType =
  | 'session_start'
  | 'session_reminder_injected'
  | 'turn_started'
  | 'message'
  | 'tool_execution'
  | 'tool_result'
  | 'turn_state'
  | 'compaction_started'
  | 'compaction_summary'
  | 'compaction_finished'
  | 'compaction_fallback'
  | 'turn_completed'
  | 'turn_failed'
  | 'turn_cancelled'
  | 'timeline_event'
  | 'token_usage'
  | 'session_settings_updated'

export interface AgentSessionEvent {
  schema_version: typeof AGENT_SESSION_SCHEMA_VERSION
  type: AgentSessionEventType
  session_id: string
  ts: number
  event_id?: string
  event_seq?: number
  dedupe_key?: string
  turn?: number
  payload?: Record<string, unknown>
}

export interface AgentSessionEventDiagnostics {
  client_request_id?: string
  bound_turn_id?: number
  hydrate_source?: string
}

export interface AgentSessionMeta {
  schema_version: typeof AGENT_SESSION_SCHEMA_VERSION
  session_id: string
  title?: string
  created_at: number
  updated_at: number
  last_turn?: number
  last_stop_reason?: 'success' | 'cancel' | 'error' | 'limit'
  active_chapter_path?: string
  compaction_count?: number
}

export type AgentSessionRuntimeState =
  | 'ready'
  | 'running'
  | 'suspended_confirmation'
  | 'suspended_askuser'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'degraded'

export type AgentSessionHydrationStatus =
  | 'memory_hit'
  | 'snapshot_loaded'
  | 'event_rebuilt'
  | 'readonly_fallback'

export type AgentSessionReadonlyReason =
  | 'runtime_state_unavailable'
  | 'historical_suspended_session_without_runtime_snapshot'
  | 'provider_credentials_unavailable_for_resume'

export interface AgentSessionHydrationAuthority {
  lastTurn?: number
  nextTurnId?: number
  sessionRevision?: number
  hydrationSource?: string
}

export interface AgentSessionSettings {
  schema_version: typeof AGENT_SESSION_SCHEMA_VERSION
  session_id: string
  model?: string
  provider?: string
  token_budget?: number
  metadata?: Record<string, unknown>
}

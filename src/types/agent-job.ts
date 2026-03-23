export type DelegateResultStatus = 'completed' | 'failed' | 'cancelled' | 'blocked'

export type DelegateStopReason =
  | 'success'
  | 'error'
  | 'cancelled'
  | 'limit'
  | 'waiting_confirmation'
  | 'waiting_askuser'
  | 'blocked'
  | 'unknown'

export type JobKind = 'mission_ad_hoc' | 'macro_workflow' | 'delegate_batch' | 'unknown'

export type JobStatus =
  | 'draft'
  | 'ready'
  | 'running'
  | 'blocked'
  | 'waiting_user'
  | 'waiting_review'
  | 'waiting_knowledge_decision'
  | 'paused'
  | 'completed'
  | 'failed'
  | 'cancelled'

export type ResourceLockKind =
  | 'file'
  | 'chapter'
  | 'canon'
  | 'review'
  | 'external_dependency'

export type ResourceLockMode = 'shared' | 'exclusive'

export interface ResourceLock {
  lock_id: string
  lock_kind: ResourceLockKind
  scope: string
  mode: ResourceLockMode
}

export interface ChangedPath {
  path: string
  change_kind: 'created' | 'modified' | 'deleted' | 'unknown'
}

export interface ArtifactRef {
  kind: string
  value: string
  description?: string | null
}

export interface EvidenceItem {
  kind: string
  summary: string
  value?: string | null
}

export interface OpenIssue {
  code?: string | null
  summary: string
  blocking: boolean
}

export interface DelegateUsage {
  rounds_executed: number
  total_tool_calls: number
  latency_ms: number
  llm_usage?: unknown | null
}

export interface DelegateResult {
  delegate_id: string
  job_id: string
  parent_task_id: string
  goal: string
  status: DelegateResultStatus
  stop_reason: DelegateStopReason
  result_summary: string
  changed_paths: ChangedPath[]
  artifacts: ArtifactRef[]
  evidence: EvidenceItem[]
  open_issues: OpenIssue[]
  next_actions: string[]
  usage?: DelegateUsage | null
  actor_id?: string | null
}

export type JobBlockerKind =
  | 'review_gate'
  | 'knowledge_decision'
  | 'user_clarification'
  | 'external_dependency'

export interface JobBlocker {
  blocker_id: string
  kind: JobBlockerKind
  summary: string
  blocking: boolean
  feature_id?: string | null
  created_at: number
  updated_at: number
}

export interface JobSnapshot {
  schema_version: number
  job_id: string
  job_kind: JobKind
  status: JobStatus
  blockers: JobBlocker[]
  ready_tasks: string[]
  running_tasks: string[]
  completed_tasks: string[]
  failed_tasks: string[]
  task_results: DelegateResult[]
  updated_at: number
}

export interface JobEvent {
  schema_version: number
  event_type: string
  job_id: string
  task_id: string
  payload: Record<string, unknown>
  ts: number
}

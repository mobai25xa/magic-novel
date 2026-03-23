/**
 * mission.ts — Tauri command bindings for the Mission system.
 *
 * Mirrors the Rust commands in src-tauri/src/commands/mission.rs.
 */

import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'

import type {
  DelegateResult,
  JobBlocker,
  JobBlockerKind,
  JobKind,
  JobSnapshot,
  JobStatus,
} from '@/types/agent-job'
import type {
  KnowledgeDecisionInput,
  KnowledgeDelta,
  KnowledgeLatest,
  KnowledgeProposalBundle,
} from '@/types/knowledge'
import type {
  MacroCreateInput,
  MacroCreateOutput,
  MacroGetStateOutput,
  MacroWorkflowConfig,
  MacroWorkflowState,
  ChapterRunState,
} from '@/types/macro-workflow'
import type {
  ReviewDecisionAnswer,
  ReviewDecisionRequest,
  ReviewReport,
} from '@/types/review'

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value)
}

function unwrapMaybeWrapped(value: unknown, key: string): unknown {
  if (!isRecord(value)) return value
  const wrapped = value[key]
  return wrapped === undefined ? value : wrapped
}

function zodErrorSummary(error: z.ZodError) {
  const head = error.issues
    .slice(0, 3)
    .map((issue) => {
      const path = issue.path.length ? issue.path.join('.') : '(root)'
      return `${path}: ${issue.message}`
    })
    .join('; ')
  return head || 'invalid payload'
}

// ── Types ────────────────────────────────────────────────────────

export interface Feature {
  id: string
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'cancelled'
  description: string
  skill: string
  preconditions: string[]
  depends_on: string[]
  expected_behavior: string[]
  verification_steps: string[]
  write_paths?: string[]
}

export interface WorkerAssignment {
  feature_id: string
  attempt: number
  started_at: number
  last_heartbeat_at: number
}

export interface StateDoc {
  schema_version: number
  mission_id: string
  state: MissionState
  current_feature_id?: string
  current_worker_id?: string
  assignments: Record<string, WorkerAssignment>
  worker_pids: Record<string, number>
  cwd: string
  updated_at: number
}

export interface FeaturesDoc {
  schema_version: number
  mission_id: string
  title: string
  features: Feature[]
}

export interface HandoffEntry {
  feature_id: string
  worker_id: string
  ok: boolean
  summary: string
  commands_run: string[]
  artifacts: string[]
  issues: string[]
}

export type TaskResultStatus = 'completed' | 'failed' | 'cancelled' | 'blocked'

export type TaskStopReason =
  | 'success'
  | 'error'
  | 'cancelled'
  | 'limit'
  | 'waiting_confirmation'
  | 'waiting_askuser'
  | 'blocked'
  | 'unknown'

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

export interface TaskUsage {
  rounds_executed: number
  total_tool_calls: number
  latency_ms: number
  llm_usage?: unknown | null
}

export interface AgentTaskResult {
  task_id: string
  actor_id: string
  goal: string
  status: TaskResultStatus
  stop_reason: TaskStopReason
  result_summary: string
  changed_paths: ChangedPath[]
  artifacts: ArtifactRef[]
  evidence: EvidenceItem[]
  open_issues: OpenIssue[]
  next_actions: string[]
  usage?: TaskUsage | null
}

export type MissionState =
  | 'awaiting_input'
  | 'initializing'
  | 'running'
  | 'blocked'
  | 'waiting_user'
  | 'waiting_review'
  | 'waiting_knowledge_decision'
  | 'paused'
  | 'orchestrator_turn'
  | 'failed'
  | 'completed'
  | 'cancelled'

export interface WorkflowDoc {
  mission_id: string
  workflow_kind: string
  creation_reason: string
  summary_job_policy: string
  status: string
  created_at: number
  updated_at: number
}

export interface WorkflowBlocker {
  kind: string
  title: string
  summary: string
  created_at?: number
  updated_at?: number
}

export interface WorkflowBlockersDoc {
  mission_id: string
  blockers: WorkflowBlocker[]
  updated_at?: number
}

export interface MissionRecoveryEntry {
  ts: number
  message: string
}

export interface MissionRecoveryLog {
  mission_id?: string
  entries: MissionRecoveryEntry[]
}

export interface MissionGetStatusOutput {
  state: StateDoc
  features: FeaturesDoc
  task_results: AgentTaskResult[]
  handoffs: HandoffEntry[]
  workflow: WorkflowDoc
  blockers: WorkflowBlockersDoc
  job_snapshot: JobSnapshot
  recovery_log?: MissionRecoveryLog | MissionRecoveryEntry[] | null
}

export type MissionResultEntry = {
  key: string
  task_id: string
  actor_id: string
  status: TaskResultStatus
  summary: string
  issues: string[]
  artifacts: string[]
  evidence: string[]
  next_actions: string[]
  commands_run: string[]
  source: 'task_result' | 'handoff'
}

const MISSION_STATE_TO_JOB_STATUS: Record<MissionState, JobStatus> = {
  awaiting_input: 'ready',
  initializing: 'running',
  running: 'running',
  blocked: 'blocked',
  waiting_user: 'waiting_user',
  waiting_review: 'waiting_review',
  waiting_knowledge_decision: 'waiting_knowledge_decision',
  paused: 'paused',
  orchestrator_turn: 'running',
  failed: 'failed',
  completed: 'completed',
  cancelled: 'cancelled',
}

function toJobStatusFromRawState(raw: unknown): JobStatus {
  const value = typeof raw === 'string' ? raw.trim() : ''
  switch (value) {
    case 'draft':
    case 'ready':
    case 'running':
    case 'blocked':
    case 'waiting_user':
    case 'waiting_review':
    case 'waiting_knowledge_decision':
    case 'paused':
    case 'completed':
    case 'failed':
    case 'cancelled':
      return value
    case 'awaiting_input':
    case 'initializing':
    case 'orchestrator_turn':
      return MISSION_STATE_TO_JOB_STATUS[value]
    default:
      return 'draft'
  }
}

function toJobKindFromRawWorkflowKind(raw: unknown): JobKind {
  const value = typeof raw === 'string' ? raw.trim() : ''
  if (value === 'ad_hoc') return 'mission_ad_hoc'
  if (value === 'macro') return 'macro_workflow'
  return 'unknown'
}

function toJobBlockerKind(raw: unknown): JobBlockerKind {
  const value = typeof raw === 'string' ? raw.trim() : ''
  switch (value) {
    case 'review_gate':
      return 'review_gate'
    case 'knowledge_decision':
      return 'knowledge_decision'
    case 'user_clarification':
      return 'user_clarification'
    default:
      return 'external_dependency'
  }
}

function toDelegateResult(
  taskResult: AgentTaskResult,
  missionId: string,
): DelegateResult {
  const actorId = typeof taskResult.actor_id === 'string' ? taskResult.actor_id.trim() : ''
  const taskId = typeof taskResult.task_id === 'string' ? taskResult.task_id.trim() : ''

  return {
    delegate_id: actorId || taskId || `delegate_${missionId || 'mission'}`,
    job_id: missionId,
    parent_task_id: taskId,
    goal: typeof taskResult.goal === 'string' ? taskResult.goal : '',
    status: taskResult.status,
    stop_reason: taskResult.stop_reason,
    result_summary: taskResult.result_summary,
    changed_paths: taskResult.changed_paths,
    artifacts: taskResult.artifacts,
    evidence: taskResult.evidence,
    open_issues: taskResult.open_issues,
    next_actions: taskResult.next_actions,
    usage: taskResult.usage,
    actor_id: actorId || null,
  }
}

function normalizeDelegateResultJobId(
  result: DelegateResult,
  defaultJobId: string,
): DelegateResult {
  const normalizedJobId = typeof result.job_id === 'string' ? result.job_id.trim() : ''
  return {
    ...result,
    job_id: normalizedJobId || defaultJobId,
  }
}

export function missionStatusToJobSnapshot(status: MissionGetStatusOutput): JobSnapshot {
  const missionId = typeof status?.state?.mission_id === 'string' ? status.state.mission_id.trim() : ''
  const stateUpdatedAt = typeof status?.state?.updated_at === 'number' ? status.state.updated_at : 0
  const workflowUpdatedAt = typeof status?.workflow?.updated_at === 'number' ? status.workflow.updated_at : 0
  const blockersUpdatedAt = typeof status?.blockers?.updated_at === 'number' ? status.blockers.updated_at : 0
  const fallbackUpdatedAt = Math.max(stateUpdatedAt, workflowUpdatedAt, blockersUpdatedAt)

  if (status?.job_snapshot && typeof status.job_snapshot.job_id === 'string') {
    const snapshot = status.job_snapshot
    const normalizedJobId = snapshot.job_id.trim() || missionId

    return {
      ...snapshot,
      job_id: normalizedJobId,
      blockers: Array.isArray(snapshot.blockers) ? snapshot.blockers : [],
      ready_tasks: Array.isArray(snapshot.ready_tasks) ? snapshot.ready_tasks : [],
      running_tasks: Array.isArray(snapshot.running_tasks) ? snapshot.running_tasks : [],
      completed_tasks: Array.isArray(snapshot.completed_tasks) ? snapshot.completed_tasks : [],
      failed_tasks: Array.isArray(snapshot.failed_tasks) ? snapshot.failed_tasks : [],
      task_results: (snapshot.task_results ?? []).map((result) => normalizeDelegateResultJobId(result, normalizedJobId)),
      updated_at: Number.isFinite(snapshot.updated_at) ? snapshot.updated_at : fallbackUpdatedAt,
    }
  }

  const schemaVersion = typeof status?.state?.schema_version === 'number' ? status.state.schema_version : 1
  const workflowStatus = status?.workflow?.status
  const stateStatus = status?.state?.state

  const readyTasks: string[] = []
  const runningTasks: string[] = []
  const completedTasks: string[] = []
  const failedTasks: string[] = []

  for (const feature of status?.features?.features ?? []) {
    const taskId = typeof feature?.id === 'string' ? feature.id.trim() : ''
    if (!taskId) continue

    if (feature.status === 'pending') {
      readyTasks.push(taskId)
      continue
    }
    if (feature.status === 'in_progress') {
      runningTasks.push(taskId)
      continue
    }
    if (feature.status === 'completed') {
      completedTasks.push(taskId)
      continue
    }
    if (feature.status === 'failed' || feature.status === 'cancelled') {
      failedTasks.push(taskId)
    }
  }

  const blockers: JobBlocker[] = (status?.blockers?.blockers ?? []).map((blocker, index) => {
    const summary = typeof blocker?.summary === 'string' ? blocker.summary : ''
    const kind = toJobBlockerKind(blocker?.kind)
    const createdAt = typeof blocker?.created_at === 'number' ? blocker.created_at : 0
    const updatedAt = typeof blocker?.updated_at === 'number' ? blocker.updated_at : createdAt

    return {
      blocker_id: `${kind}_${index}`,
      kind,
      summary,
      blocking: true,
      created_at: createdAt,
      updated_at: updatedAt,
    }
  })
  const updatedAt = fallbackUpdatedAt

  return {
    schema_version: schemaVersion,
    job_id: missionId,
    job_kind: toJobKindFromRawWorkflowKind(status?.workflow?.workflow_kind),
    status: toJobStatusFromRawState(workflowStatus ?? stateStatus),
    blockers,
    ready_tasks: readyTasks,
    running_tasks: runningTasks,
    completed_tasks: completedTasks,
    failed_tasks: failedTasks,
    task_results: (status.task_results ?? []).map((result) => toDelegateResult(result, missionId)),
    updated_at: updatedAt,
  }
}

export function getMissionResultEntriesFromJobSnapshot(
  snapshot: Pick<JobSnapshot, 'task_results'>,
): MissionResultEntry[] {
  return (snapshot.task_results ?? []).map((result, index) => {
    const actorId = String(result.actor_id ?? result.delegate_id ?? '').trim()
    const taskId = String(result.parent_task_id ?? '').trim()
    const summary = result.result_summary.trim() || normalizeTaskResultSummary({
      status: result.status,
      result_summary: result.result_summary,
    })

    return {
      key: `${actorId || 'worker'}-${taskId || 'task'}-${index}`,
      task_id: taskId,
      actor_id: actorId,
      status: result.status,
      summary,
      issues: dedupeTrimmedStrings((result.open_issues ?? []).map((issue) => String(issue?.summary ?? ''))),
      artifacts: dedupeTrimmedStrings(
        result.artifacts?.length
          ? result.artifacts.map((artifact) => String(artifact?.value ?? ''))
          : result.changed_paths.map((changedPath) => String(changedPath?.path ?? '')),
      ),
      evidence: dedupeTrimmedStrings((result.evidence ?? []).map((item) => String(item?.summary ?? ''))),
      next_actions: dedupeTrimmedStrings((result.next_actions ?? []).map((value) => String(value ?? ''))),
      commands_run: [],
      source: 'task_result',
    }
  })
}

function dedupeTrimmedStrings(values: string[]) {
  const result: string[] = []
  const seen = new Set<string>()

  for (const value of values) {
    const normalized = value.trim()
    if (!normalized || seen.has(normalized)) {
      continue
    }
    seen.add(normalized)
    result.push(normalized)
  }

  return result
}

function toMissionRecoveryEntry(value: unknown): MissionRecoveryEntry | null {
  if (typeof value === 'string') {
    const message = value.trim()
    return message ? { ts: 0, message } : null
  }

  if (!isRecord(value)) {
    return null
  }

  const rawTs = value.ts
  const ts = typeof rawTs === 'number'
    ? rawTs
    : Number(String(rawTs ?? ''))
  const rawMessage = value.message
  const message = typeof rawMessage === 'string'
    ? rawMessage.trim()
    : ''

  if (!message) {
    return null
  }

  return {
    ts: Number.isFinite(ts) ? ts : 0,
    message,
  }
}

export function getMissionRecoveryEntries(
  status: Pick<MissionGetStatusOutput, 'recovery_log'> | null | undefined,
): MissionRecoveryEntry[] {
  const raw = status?.recovery_log
  const rawEntries = Array.isArray(raw)
    ? raw
    : isRecord(raw) && Array.isArray(raw.entries)
      ? raw.entries
      : []

  return rawEntries
    .map((entry) => toMissionRecoveryEntry(entry))
    .filter((entry): entry is MissionRecoveryEntry => entry != null)
    .sort((left, right) => right.ts - left.ts)
}

export function normalizeTaskResultSummary(
  result: Pick<AgentTaskResult, 'status' | 'result_summary'>,
) {
  const summary = result.result_summary.trim()
  if (summary) {
    return summary
  }

  switch (result.status) {
    case 'completed':
      return 'task completed'
    case 'failed':
      return 'task failed'
    case 'cancelled':
      return 'task cancelled'
    case 'blocked':
      return 'task blocked'
    default:
      return 'task finished'
  }
}

export function isTaskResultSuccessful(
  result: Pick<AgentTaskResult, 'status'>,
) {
  return result.status === 'completed'
}

export function getMissionResultEntries(
  status: Pick<MissionGetStatusOutput, 'task_results' | 'handoffs'>,
): MissionResultEntry[] {
  if (Array.isArray(status.task_results) && status.task_results.length > 0) {
    return status.task_results.map((result, index) => ({
      key: `${result.actor_id}-${result.task_id}-${index}`,
      task_id: result.task_id,
      actor_id: result.actor_id,
      status: result.status,
      summary: normalizeTaskResultSummary(result),
      issues: dedupeTrimmedStrings(
        (result.open_issues ?? []).map((issue) => String(issue?.summary ?? '')),
      ),
      artifacts: dedupeTrimmedStrings(
        (result.artifacts?.length
          ? result.artifacts.map((artifact) => String(artifact?.value ?? ''))
          : result.changed_paths.map((changedPath) => String(changedPath?.path ?? ''))),
      ),
      evidence: dedupeTrimmedStrings(
        (result.evidence ?? []).map((item) => String(item?.summary ?? '')),
      ),
      next_actions: dedupeTrimmedStrings(
        (result.next_actions ?? []).map((value) => String(value ?? '')),
      ),
      commands_run: [],
      source: 'task_result',
    }))
  }

  return (status.handoffs ?? []).map((handoff, index) => ({
    key: `${handoff.worker_id}-${handoff.feature_id}-${index}`,
    task_id: handoff.feature_id,
    actor_id: handoff.worker_id,
    status: handoff.ok ? 'completed' : 'failed',
    summary: handoff.summary?.trim() || (handoff.ok ? 'task completed' : 'task failed'),
    issues: dedupeTrimmedStrings((handoff.issues ?? []).map((value) => String(value ?? ''))),
    artifacts: dedupeTrimmedStrings((handoff.artifacts ?? []).map((value) => String(value ?? ''))),
    evidence: [],
    next_actions: [],
    commands_run: dedupeTrimmedStrings(
      (handoff.commands_run ?? []).map((value) => String(value ?? '')),
    ),
    source: 'handoff',
  }))
}

export function getMissionResultEntriesPreferJobSnapshot(status: MissionGetStatusOutput): MissionResultEntry[] {
  const fromSnapshot = getMissionResultEntriesFromJobSnapshot(missionStatusToJobSnapshot(status))
  if (fromSnapshot.length > 0) {
    return fromSnapshot
  }

  return getMissionResultEntries(status)
}

export interface MissionCreateInput {
  project_path: string
  title: string
  mission_text: string
  features: Feature[]
}

export type MissionDelegateTransport = 'process' | 'in_process'

export interface MissionStartInput {
  project_path: string
  mission_id: string
  max_workers?: number
  model?: string
  provider?: string
  base_url?: string
  api_key?: string
  parent_session_id?: string
  parent_turn_id?: number
  delegate_transport?: MissionDelegateTransport
}

export interface MissionCreateOutput {
  schema_version: number
  mission_id: string
}

// ── Commands ─────────────────────────────────────────────────────

export async function missionCreate(
  input: MissionCreateInput,
): Promise<MissionCreateOutput> {
  return invoke<MissionCreateOutput>('mission_create', { input })
}

export async function missionList(projectPath: string): Promise<string[]> {
  return invoke<string[]>('mission_list', { input: { project_path: projectPath } })
}

export async function missionGetStatus(
  projectPath: string,
  missionId: string,
): Promise<MissionGetStatusOutput> {
  return invoke<MissionGetStatusOutput>('mission_get_status', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionStart(input: MissionStartInput): Promise<void> {
  return invoke<void>('mission_start', { input })
}

export async function missionPause(
  projectPath: string,
  missionId: string,
): Promise<void> {
  return invoke<void>('mission_pause', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionResume(
  projectPath: string,
  missionId: string,
): Promise<void> {
  return invoke<void>('mission_resume', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionCancel(
  projectPath: string,
  missionId: string,
): Promise<void> {
  return invoke<void>('mission_cancel', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

// ── M3 Review Gate ───────────────────────────────────────────

const ReviewIssueSchema = z
  .object({
    issue_id: z.string(),
    review_type: z.string(),
    severity: z.string(),
    summary: z.string(),
    subject_refs: z.array(z.string()).default([]),
    evidence_refs: z.array(z.string()).default([]),
    confidence: z.string(),
    suggested_fix: z.string().optional(),
    auto_fixable: z.boolean(),
  })
  .passthrough()

const ReviewReportSchema = z
  .object({
    schema_version: z.number(),
    review_id: z.string(),
    scope_ref: z.string(),
    target_refs: z.array(z.string()).default([]),
    review_types: z.array(z.string()).default([]),
    overall_status: z.string(),
    issues: z.array(ReviewIssueSchema).default([]),
    evidence_summary: z.array(z.string()).default([]),
    recommended_action: z.string(),
    generated_at: z.number(),
  })
  .passthrough()

const ReviewDecisionRequestSchema = z
  .object({
    schema_version: z.number(),
    review_id: z.string(),
    feature_id: z.string().optional().nullable(),
    scope_ref: z.string(),
    target_refs: z.array(z.string()).optional().nullable(),
    question: z.string(),
    options: z.array(z.string()).default([]),
    context_summary: z.array(z.string()).default([]),
    created_at: z.number(),
  })
  .passthrough()

function parseReviewReport(value: unknown): ReviewReport | null {
  if (value == null) return null
  const parsed = ReviewReportSchema.safeParse(value)
  if (parsed.success) return parsed.data as ReviewReport
  console.warn(`[mission] review report schema mismatch:`, zodErrorSummary(parsed.error))
  return null
}

function parseReviewReports(value: unknown): ReviewReport[] {
  if (value == null) return []
  const parsed = z.array(ReviewReportSchema).safeParse(value)
  if (parsed.success) return parsed.data as ReviewReport[]
  console.warn(`[mission] review list schema mismatch:`, zodErrorSummary(parsed.error))
  return []
}

function parseReviewDecisionRequest(value: unknown): ReviewDecisionRequest | null {
  if (value == null) return null
  const parsed = ReviewDecisionRequestSchema.safeParse(value)
  if (parsed.success) return parsed.data as ReviewDecisionRequest
  console.warn(`[mission] review decision schema mismatch:`, zodErrorSummary(parsed.error))
  return null
}

function unwrapReviewReportPayload(raw: unknown): unknown {
  let value = raw
  value = unwrapMaybeWrapped(value, 'review')
  value = unwrapMaybeWrapped(value, 'review_report')
  value = unwrapMaybeWrapped(value, 'report')
  value = unwrapMaybeWrapped(value, 'latest')
  return value
}

function unwrapReviewReportsPayload(raw: unknown): unknown {
  let value = raw
  value = unwrapMaybeWrapped(value, 'reviews')
  value = unwrapMaybeWrapped(value, 'reports')
  value = unwrapMaybeWrapped(value, 'list')
  return value
}

function unwrapReviewDecisionPayload(raw: unknown): unknown {
  let value = raw
  value = unwrapMaybeWrapped(value, 'pending_decision')
  value = unwrapMaybeWrapped(value, 'decision')
  return value
}

export async function missionReviewGetLatest(
  projectPath: string,
  missionId: string,
): Promise<ReviewReport | null> {
  const raw = await invoke<unknown>('mission_review_get_latest', {
    input: { project_path: projectPath, mission_id: missionId },
  })

  const resolved = unwrapReviewReportPayload(raw)
  return parseReviewReport(resolved)
}

export async function missionReviewList(
  projectPath: string,
  missionId: string,
): Promise<ReviewReport[]> {
  const raw = await invoke<unknown>('mission_review_list', {
    input: { project_path: projectPath, mission_id: missionId },
  })

  const resolved = unwrapReviewReportsPayload(raw)
  return parseReviewReports(resolved)
}

export async function missionReviewGetPendingDecision(
  projectPath: string,
  missionId: string,
): Promise<ReviewDecisionRequest | null> {
  const raw = await invoke<unknown>('mission_review_get_pending_decision', {
    input: { project_path: projectPath, mission_id: missionId },
  })

  const resolved = unwrapReviewDecisionPayload(raw)
  return parseReviewDecisionRequest(resolved)
}

export interface MissionReviewAnswerInput {
  project_path: string
  mission_id: string
  answer: ReviewDecisionAnswer
}

export async function missionReviewAnswer(input: MissionReviewAnswerInput): Promise<void> {
  await invoke<void>('mission_review_answer', { input })
}

// ── M4 Knowledge Writeback / Canon Gate ─────────────────────

const KnowledgeConflictSchema = z
  .object({
    type: z.string(),
    message: z.string(),
    item_id: z.string().optional(),
    target_ref: z.string().optional(),
  })
  .passthrough()

const KnowledgeProposalItemSchema = z
  .object({
    item_id: z.string(),
    kind: z.string(),
    op: z.string(),
    target_ref: z.string().optional(),
    target_revision: z.number().optional(),
    fields: z.record(z.string(), z.unknown()).default({}),
    evidence_refs: z.array(z.string()).default([]),
    source_refs: z.array(z.string()).default([]),
    change_reason: z.string().default(''),
    accept_policy: z.string(),
  })
  .passthrough()

const KnowledgeProposalBundleSchema = z
  .object({
    schema_version: z.number(),
    bundle_id: z.string(),
    scope_ref: z.string(),
    branch_id: z.string().optional(),
    source_session_id: z.string(),
    source_review_id: z.string().optional(),
    generated_at: z.number(),
    proposal_items: z.array(KnowledgeProposalItemSchema).default([]),
  })
  .passthrough()

const KnowledgeDeltaTargetSchema = z
  .object({
    ref: z.string(),
    kind: z.string(),
    path: z.string().optional(),
  })
  .passthrough()

const KnowledgeDeltaChangeSchema = z
  .object({
    item_id: z.string(),
    op: z.string(),
    kind: z.string(),
    target_ref: z.string().optional(),
    summary: z.string(),
  })
  .passthrough()

const KnowledgeRollbackInfoSchema = z
  .object({
    kind: z.string(),
    token: z.string().optional(),
  })
  .passthrough()

const KnowledgeDeltaSchema = z
  .object({
    schema_version: z.number(),
    knowledge_delta_id: z.string(),
    status: z.string(),
    scope_ref: z.string(),
    branch_id: z.string().optional(),
    source_session_id: z.string(),
    source_review_id: z.string().optional(),
    generated_at: z.number(),
    targets: z.array(KnowledgeDeltaTargetSchema).default([]),
    changes: z.array(KnowledgeDeltaChangeSchema).default([]),
    evidence_refs: z.array(z.string()).default([]),
    conflicts: z.array(KnowledgeConflictSchema).default([]),
    accepted_item_ids: z.array(z.string()).optional(),
    rejected_item_ids: z.array(z.string()).optional(),
    applied_at: z.number().optional(),
    rollback: KnowledgeRollbackInfoSchema.optional(),
  })
  .passthrough()

function parseKnowledgeProposalBundle(value: unknown): KnowledgeProposalBundle | null {
  if (value == null) return null
  const parsed = KnowledgeProposalBundleSchema.safeParse(value)
  if (parsed.success) return parsed.data as KnowledgeProposalBundle
  console.warn(`[mission] knowledge bundle schema mismatch:`, zodErrorSummary(parsed.error))
  return null
}

function parseKnowledgeDelta(value: unknown): KnowledgeDelta | null {
  if (value == null) return null
  const parsed = KnowledgeDeltaSchema.safeParse(value)
  if (parsed.success) return parsed.data as KnowledgeDelta
  console.warn(`[mission] knowledge delta schema mismatch:`, zodErrorSummary(parsed.error))
  return null
}

function unwrapKnowledgeLatestPayload(raw: unknown): unknown {
  let value = raw
  value = unwrapMaybeWrapped(value, 'knowledge')
  value = unwrapMaybeWrapped(value, 'latest')
  return value
}

export async function missionKnowledgeGetLatest(
  projectPath: string,
  missionId: string,
): Promise<KnowledgeLatest> {
  const raw = await invoke<unknown>('mission_knowledge_get_latest', {
    input: { project_path: projectPath, mission_id: missionId },
  })

  const resolved = unwrapKnowledgeLatestPayload(raw)
  if (!isRecord(resolved)) {
    return { bundle: null, delta: null }
  }

  return {
    bundle: parseKnowledgeProposalBundle(resolved.bundle),
    delta: parseKnowledgeDelta(resolved.delta),
  }
}

export interface MissionKnowledgeDecideInput {
  project_path: string
  mission_id: string
  decision: KnowledgeDecisionInput
}

export async function missionKnowledgeDecide(input: MissionKnowledgeDecideInput): Promise<void> {
  await invoke<void>('mission_knowledge_decide', { input })
}

export async function missionKnowledgeApply(
  projectPath: string,
  missionId: string,
): Promise<void> {
  await invoke<void>('mission_knowledge_apply', {
    input: { project_path: projectPath, mission_id: missionId },
  })
}

export async function missionKnowledgeRollback(
  projectPath: string,
  missionId: string,
  token?: string,
): Promise<void> {
  await invoke<void>('mission_knowledge_rollback', {
    input: { project_path: projectPath, mission_id: missionId, token },
  })
}

// ── M2 Layer1 / ContextPack ────────────────────────────────────

export type Layer1WorkflowKind = 'micro' | 'chapter' | 'arc' | 'book' | 'knowledge'

const Layer1WorkflowKindSchema = z.enum(['micro', 'chapter', 'arc', 'book', 'knowledge'])

const ChapterCardSchema = z
  .object({
    schema_version: z.number(),
    scope_ref: z.string(),
    scope_locator: z.string().optional(),
    objective: z.string(),
    workflow_kind: Layer1WorkflowKindSchema,
    target_refs: z.array(z.string()).optional(),
    must_keep: z.array(z.string()).optional(),
    hard_constraints: z.array(z.string()).default([]),
    success_criteria: z.array(z.string()).default([]),
    review_targets: z.array(z.string()).optional(),
    writeback_targets: z.array(z.string()).optional(),
    status: z.enum(['draft', 'active', 'blocked', 'completed']),
    source_session_id: z.string().optional(),
    source_rules_fingerprint: z.string().optional(),
    branch_id: z.string().optional(),
    ref: z.string().optional(),
    updated_at: z.number(),
  })
  .passthrough()

const RecentFactSchema = z
  .object({
    fact_ref: z.string().optional(),
    summary: z.string(),
    source_ref: z.string(),
    confidence: z.enum(['accepted', 'proposed']),
    branch_id: z.string().optional(),
  })
  .passthrough()

const RecentFactsSchema = z
  .object({
    schema_version: z.number(),
    scope_ref: z.string(),
    ref: z.string().optional(),
    branch_id: z.string().optional(),
    facts: z.array(RecentFactSchema).default([]),
    updated_at: z.number(),
  })
  .passthrough()

const CastMemberSchema = z
  .object({
    character_ref: z.string(),
    role_in_scope: z.string().optional(),
    current_state_summary: z.string(),
    must_keep_voice_signals: z.array(z.string()).optional(),
    sensitivity_flags: z.array(z.string()).optional(),
  })
  .passthrough()

const ActiveCastSchema = z
  .object({
    schema_version: z.number(),
    scope_ref: z.string(),
    ref: z.string().optional(),
    cast: z.array(CastMemberSchema).default([]),
    updated_at: z.number(),
  })
  .passthrough()

const ActiveForeshadowingItemSchema = z
  .object({
    foreshadow_ref: z.string(),
    status: z.enum(['seeded', 'active', 'partially_paid', 'paid', 'stalled']),
    required_action: z.string().optional(),
    evidence_ref: z.string().optional(),
  })
  .passthrough()

const ActiveForeshadowingSchema = z
  .object({
    schema_version: z.number(),
    scope_ref: z.string(),
    ref: z.string().optional(),
    items: z.array(ActiveForeshadowingItemSchema).default([]),
    updated_at: z.number(),
  })
  .passthrough()

const PreviousSummarySchema = z
  .object({
    schema_version: z.number(),
    scope_ref: z.string(),
    ref: z.string().optional(),
    related_chapter_refs: z.array(z.string()).default([]),
    summary: z.string(),
    critical_carryovers: z.array(z.string()).default([]),
    updated_at: z.number(),
  })
  .passthrough()

const RiskLedgerItemSchema = z
  .object({
    risk_id: z.string(),
    severity: z.enum(['info', 'warn', 'block']),
    summary: z.string(),
    source: z.enum(['review', 'user', 'orchestrator', 'knowledge']),
    status: z.enum(['open', 'deferred', 'resolved']),
    evidence_refs: z.array(z.string()).optional(),
  })
  .passthrough()

const RiskLedgerSchema = z
  .object({
    schema_version: z.number(),
    scope_ref: z.string(),
    ref: z.string().optional(),
    items: z.array(RiskLedgerItemSchema).default([]),
    updated_at: z.number(),
  })
  .passthrough()

const ContextPackCastNoteSchema = z
  .object({
    character_ref: z.string(),
    summary: z.string(),
    voice_signals: z.array(z.string()).optional(),
  })
  .passthrough()

const ContextPackEvidenceSchema = z
  .object({
    source_ref: z.string(),
    snippet: z.string(),
    reason: z.string(),
    score: z.number(),
  })
  .passthrough()

const ContextPackRevisionSchema = z
  .object({
    ref: z.string(),
    revision: z.number(),
  })
  .passthrough()

const ContextPackSchema = z
  .object({
    schema_version: z.number().optional(),
    ref: z.string().optional(),
    task_card_ref: z.string().optional(),
    scope_ref: z.string(),
    branch_id: z.string().optional(),
    token_budget: z.enum(['small', 'medium', 'large']),
    objective_summary: z.string(),
    must_keep: z.array(z.string()).default([]),
    active_constraints: z.array(z.string()).default([]),
    key_facts: z.array(z.string()).default([]),
    cast_notes: z.array(ContextPackCastNoteSchema).default([]),
    active_foreshadowing: z
      .array(
        z
          .object({
            foreshadow_ref: z.string(),
            summary: z.string(),
            required_action: z.string().optional(),
          })
          .passthrough(),
      )
      .optional(),
    evidence_snippets: z.array(ContextPackEvidenceSchema).default([]),
    style_rules: z.array(z.string()).default([]),
    review_targets: z.array(z.string()).default([]),
    risk_flags: z.array(z.string()).default([]),
    source_revisions: z.array(ContextPackRevisionSchema).default([]),
    generated_at: z.number(),
  })
  .passthrough()

function parseLayer1Doc<T>(schema: z.ZodType<T>, value: unknown, label: string): T | null {
  if (value == null) return null
  const parsed = schema.safeParse(value)
  if (parsed.success) return parsed.data
  console.warn(`[mission] ${label} schema mismatch:`, zodErrorSummary(parsed.error))
  return null
}

function parseContextPack(value: unknown): ContextPack {
  const parsed = ContextPackSchema.safeParse(value)
  if (parsed.success) {
    return parsed.data as ContextPack
  }

  throw new Error(`E_M2_SCHEMA_MISMATCH: contextpack (${zodErrorSummary(parsed.error)})`)
}

export interface ChapterCard {
  schema_version: number
  scope_ref: string
  scope_locator?: string
  objective: string
  workflow_kind: Layer1WorkflowKind
  target_refs?: string[]
  must_keep?: string[]
  hard_constraints: string[]
  success_criteria: string[]
  review_targets?: string[]
  writeback_targets?: string[]
  status: 'draft' | 'active' | 'blocked' | 'completed'
  source_session_id?: string
  source_rules_fingerprint?: string
  branch_id?: string
  ref?: string
  updated_at: number
}

export interface RecentFacts {
  schema_version: number
  scope_ref: string
  ref?: string
  branch_id?: string
  facts: Array<{
    fact_ref?: string
    summary: string
    source_ref: string
    confidence: 'accepted' | 'proposed'
    branch_id?: string
  }>
  updated_at: number
}

export interface ActiveCast {
  schema_version: number
  scope_ref: string
  ref?: string
  cast: Array<{
    character_ref: string
    role_in_scope?: string
    current_state_summary: string
    must_keep_voice_signals?: string[]
    sensitivity_flags?: string[]
  }>
  updated_at: number
}

export interface ActiveForeshadowing {
  schema_version: number
  scope_ref: string
  ref?: string
  items: Array<{
    foreshadow_ref: string
    status: 'seeded' | 'active' | 'partially_paid' | 'paid' | 'stalled'
    required_action?: string
    evidence_ref?: string
  }>
  updated_at: number
}

export interface PreviousSummary {
  schema_version: number
  scope_ref: string
  ref?: string
  related_chapter_refs: string[]
  summary: string
  critical_carryovers: string[]
  updated_at: number
}

export interface RiskLedger {
  schema_version: number
  scope_ref: string
  ref?: string
  items: Array<{
    risk_id: string
    severity: 'info' | 'warn' | 'block'
    summary: string
    source: 'review' | 'user' | 'orchestrator' | 'knowledge'
    status: 'open' | 'deferred' | 'resolved'
    evidence_refs?: string[]
  }>
  updated_at: number
}

export interface ContextPack {
  schema_version?: number
  ref?: string
  task_card_ref?: string
  scope_ref: string
  branch_id?: string
  token_budget: 'small' | 'medium' | 'large'
  objective_summary: string
  must_keep: string[]
  active_constraints: string[]
  key_facts: string[]
  cast_notes: Array<{
    character_ref: string
    summary: string
    voice_signals?: string[]
  }>
  active_foreshadowing?: Array<{
    foreshadow_ref: string
    summary: string
    required_action?: string
  }>
  evidence_snippets: Array<{
    source_ref: string
    snippet: string
    reason: string
    score: number
  }>
  style_rules: string[]
  review_targets: string[]
  risk_flags: string[]
  source_revisions: Array<{
    ref: string
    revision: number
  }>
  generated_at: number
}

export interface MissionLayer1Snapshot {
  chapter_card?: ChapterCard | null
  recent_facts?: RecentFacts | null
  active_cast?: ActiveCast | null
  active_foreshadowing?: ActiveForeshadowing | null
  previous_summary?: PreviousSummary | null
  risk_ledger?: RiskLedger | null
}

export type Layer1ArtifactKind =
  | 'chapter_card'
  | 'recent_facts'
  | 'active_cast'
  | 'active_foreshadowing'
  | 'previous_summary'
  | 'risk_ledger'

export type Layer1ArtifactDoc =
  | ChapterCard
  | RecentFacts
  | ActiveCast
  | ActiveForeshadowing
  | PreviousSummary
  | RiskLedger

export interface MissionLayer1UpsertInput {
  project_path: string
  mission_id: string
  kind: Layer1ArtifactKind
  doc: Layer1ArtifactDoc
}

export async function missionLayer1Get(
  projectPath: string,
  missionId: string,
): Promise<MissionLayer1Snapshot> {
  const raw = await invoke<unknown>('mission_layer1_get', {
    input: { project_path: projectPath, mission_id: missionId },
  })

  const resolved = unwrapMaybeWrapped(raw, 'layer1')
  if (!isRecord(resolved)) {
    return {}
  }

  return {
    chapter_card: parseLayer1Doc(ChapterCardSchema, resolved.chapter_card, 'chapter_card'),
    recent_facts: parseLayer1Doc(RecentFactsSchema, resolved.recent_facts, 'recent_facts'),
    active_cast: parseLayer1Doc(ActiveCastSchema, resolved.active_cast, 'active_cast'),
    active_foreshadowing: parseLayer1Doc(
      ActiveForeshadowingSchema,
      resolved.active_foreshadowing,
      'active_foreshadowing',
    ),
    previous_summary: parseLayer1Doc(
      PreviousSummarySchema,
      resolved.previous_summary,
      'previous_summary',
    ),
    risk_ledger: parseLayer1Doc(RiskLedgerSchema, resolved.risk_ledger, 'risk_ledger'),
  }
}

export async function missionLayer1Upsert(input: MissionLayer1UpsertInput): Promise<void> {
  await invoke<void>('mission_layer1_upsert', { input })
}

export async function missionContextpackGetLatest(
  projectPath: string,
  missionId: string,
): Promise<ContextPack | null> {
  const raw = await invoke<unknown>('mission_contextpack_get_latest', {
    input: { project_path: projectPath, mission_id: missionId },
  })

  const resolved = unwrapMaybeWrapped(raw, 'contextpack')
  if (resolved == null) return null
  return parseContextPack(resolved)
}

export interface MissionContextpackBuildInput {
  project_path: string
  mission_id: string
  scope_ref?: string
  token_budget?: ContextPack['token_budget']
}

export async function missionContextpackBuild(
  input: MissionContextpackBuildInput,
): Promise<ContextPack> {
  const raw = await invoke<unknown>('mission_contextpack_build', { input })
  const resolved = unwrapMaybeWrapped(raw, 'contextpack')
  return parseContextPack(resolved)
}

// ── M5: Macro Workflow Commands ─────────────────────────────────

function parseChapterRunState(raw: unknown): ChapterRunState | null {
  if (!isRecord(raw)) return null
  const r = raw as Record<string, unknown>
  const chapterRef = typeof r.chapter_ref === 'string' ? r.chapter_ref : undefined
  const writePath = typeof r.write_path === 'string' ? r.write_path : undefined
  if (!chapterRef || !writePath) return null
  return {
    ...r,
    chapter_ref: chapterRef,
    write_path: writePath,
    display_title: typeof r.display_title === 'string' ? r.display_title : undefined,
    status: typeof r.status === 'string' ? r.status as ChapterRunState['status'] : 'pending',
    stage: typeof r.stage === 'string' ? r.stage as ChapterRunState['stage'] : undefined,
    latest_contextpack_ref: typeof r.latest_contextpack_ref === 'string' ? r.latest_contextpack_ref : undefined,
    latest_review_id: typeof r.latest_review_id === 'string' ? r.latest_review_id : undefined,
    latest_knowledge_delta_id: typeof r.latest_knowledge_delta_id === 'string' ? r.latest_knowledge_delta_id : undefined,
    last_result_summary:
      typeof r.last_result_summary === 'string'
        ? r.last_result_summary
        : typeof r.last_handoff_summary === 'string'
          ? r.last_handoff_summary
          : undefined,
    updated_at: typeof r.updated_at === 'number' ? r.updated_at : 0,
  }
}

function parseMacroWorkflowConfig(raw: unknown): MacroWorkflowConfig | null {
  if (!isRecord(raw)) return null
  const r = raw as Record<string, unknown>
  if (typeof r.macro_id !== 'string' || typeof r.mission_id !== 'string') return null
  const missionId = r.mission_id
  const jobId = typeof r.job_id === 'string' ? r.job_id : missionId
  return {
    ...r,
    schema_version: typeof r.schema_version === 'number' ? r.schema_version : 1,
    macro_id: r.macro_id,
    mission_id: missionId,
    job_id: jobId,
    workflow_kind: r.workflow_kind === 'volume' ? 'volume' : 'book',
    objective: typeof r.objective === 'string' ? r.objective : '',
    chapter_targets: Array.isArray(r.chapter_targets) ? r.chapter_targets : [],
    strict_review: r.strict_review === true,
    auto_fix_on_block: r.auto_fix_on_block === true,
    token_budget: r.token_budget === 'small' ? 'small' : r.token_budget === 'large' ? 'large' : 'medium',
    created_at: typeof r.created_at === 'number' ? r.created_at : 0,
  } as MacroWorkflowConfig
}

function parseMacroWorkflowState(raw: unknown): MacroWorkflowState | null {
  if (!isRecord(raw)) return null
  const r = raw as Record<string, unknown>
  if (typeof r.macro_id !== 'string' || typeof r.mission_id !== 'string') return null
  const missionId = r.mission_id
  const jobId = typeof r.job_id === 'string' ? r.job_id : missionId
  const chapters = Array.isArray(r.chapters)
    ? r.chapters.map(parseChapterRunState).filter((c): c is ChapterRunState => c !== null)
    : []
  return {
    ...r,
    schema_version: typeof r.schema_version === 'number' ? r.schema_version : 1,
    macro_id: r.macro_id,
    mission_id: missionId,
    job_id: jobId,
    objective: typeof r.objective === 'string' ? r.objective : '',
    workflow_kind: r.workflow_kind === 'volume' ? 'volume' : 'book',
    current_index: typeof r.current_index === 'number' ? r.current_index : -1,
    current_stage: typeof r.current_stage === 'string' ? r.current_stage as MacroWorkflowState['current_stage'] : 'planning',
    chapters,
    last_transition_at: typeof r.last_transition_at === 'number' ? r.last_transition_at : 0,
    last_error: isRecord(r.last_error) ? r.last_error as MacroWorkflowState['last_error'] : undefined,
  } as MacroWorkflowState
}

export async function missionMacroCreate(
  input: MacroCreateInput,
): Promise<MacroCreateOutput> {
  return invoke<MacroCreateOutput>('mission_macro_create', { input })
}

export async function missionMacroGetState(
  projectPath: string,
  missionId: string,
): Promise<MacroGetStateOutput> {
  try {
    const raw = await invoke<unknown>('mission_macro_get_state', {
      input: { project_path: projectPath, mission_id: missionId },
    })

    if (!isRecord(raw)) {
      return { config: null, state: null }
    }

    const r = raw as Record<string, unknown>
    return {
      config: parseMacroWorkflowConfig(unwrapMaybeWrapped(r.config, 'config')),
      state: parseMacroWorkflowState(unwrapMaybeWrapped(r.state, 'state')),
    }
  } catch (error) {
    // Tolerate missing macro data — the mission may not have a macro workflow
    const text = String((error as { message?: unknown } | null)?.message ?? error ?? '').toLowerCase()
    if (text.includes('not found') || text.includes('no such file') || text.includes('os error 2')) {
      return { config: null, state: null }
    }
    throw error
  }
}

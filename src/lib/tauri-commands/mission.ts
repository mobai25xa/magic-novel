/**
 * mission.ts — Tauri command bindings for the Mission system.
 *
 * Mirrors the Rust commands in src-tauri/src/commands/mission.rs.
 */

import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'

import type { ReviewDecisionRequest, ReviewReport } from '@/types/review'
import type {
  KnowledgeDecisionInput,
  KnowledgeDelta,
  KnowledgeLatest,
  KnowledgeProposalBundle,
} from '@/types/knowledge'

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

export type MissionState =
  | 'awaiting_input'
  | 'initializing'
  | 'running'
  | 'paused'
  | 'orchestrator_turn'
  | 'completed'

export interface MissionGetStatusOutput {
  state: StateDoc
  features: FeaturesDoc
  handoffs: HandoffEntry[]
}

export interface MissionCreateInput {
  project_path: string
  title: string
  mission_text: string
  features: Feature[]
}

export interface MissionStartInput {
  project_path: string
  mission_id: string
  max_workers?: number
  model?: string
  provider?: string
  base_url?: string
  api_key?: string
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

// ── M3 Review Gate ─────────────────────────────────────────────

const ReviewTypeSchema = z.enum([
  'word_count',
  'continuity',
  'logic',
  'character',
  'style',
  'terminology',
  'foreshadow',
  'objective_completion',
])

const ReviewIssueSchema = z
  .object({
    issue_id: z.string(),
    review_type: ReviewTypeSchema,
    severity: z.enum(['info', 'warn', 'block']),
    summary: z.string(),
    subject_refs: z.array(z.string()).default([]),
    evidence_refs: z.array(z.string()).default([]),
    confidence: z.enum(['low', 'medium', 'high']),
    suggested_fix: z.string().optional(),
    auto_fixable: z.boolean(),
  })
  .passthrough()

const ReviewReportSchema = z
  .object({
    schema_version: z.number().optional(),
    review_id: z.string(),
    scope_ref: z.string(),
    target_refs: z.array(z.string()).default([]),
    review_types: z.array(ReviewTypeSchema).default([]),
    overall_status: z.enum(['pass', 'warn', 'block']),
    issues: z.array(ReviewIssueSchema).default([]),
    evidence_summary: z.array(z.string()).default([]),
    recommended_action: z.enum(['accept', 'revise', 'escalate']),
    generated_at: z.number(),
  })
  .passthrough()

const ReviewDecisionOptionSchema = z
  .object({
    option_id: z.string(),
    label: z.string(),
    description: z.string().optional(),
  })
  .passthrough()

const ReviewDecisionRequestSchema = z
  .object({
    schema_version: z.number().optional(),
    review_id: z.string().optional(),
    question: z.string(),
    options: z.array(ReviewDecisionOptionSchema).default([]),
    context_summary: z.string().optional(),
    created_at: z.number().optional(),
  })
  .passthrough()

function parseReviewReport(value: unknown): ReviewReport | null {
  if (value == null) return null
  const parsed = ReviewReportSchema.safeParse(value)
  if (parsed.success) return parsed.data as ReviewReport
  console.warn(`[mission] review schema mismatch:`, zodErrorSummary(parsed.error))
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
  review_id?: string
  answer: unknown
}

export async function missionReviewAnswer(input: MissionReviewAnswerInput): Promise<void> {
  await invoke<void>('mission_review_answer', { input })
}

export async function missionReviewFixupStart(
  projectPath: string,
  missionId: string,
): Promise<void> {
  await invoke<void>('mission_review_fixup_start', {
    input: { project_path: projectPath, mission_id: missionId },
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

// ── M4 Knowledge Writeback ────────────────────────────────────

const KnowledgeStringArraySchema = z.preprocess(
  (value) => (Array.isArray(value) ? value.map((v) => String(v)) : []),
  z.array(z.string()),
)

const KnowledgeRecordSchema = z.preprocess(
  (value) => (isRecord(value) ? value : {}),
  z.record(z.string(), z.unknown()),
)

const KnowledgeItemOpSchema = z.unknown().transform((value): 'create' | 'update' | 'archive' | 'restore' => {
  switch (String(value ?? '').trim()) {
    case 'create':
      return 'create'
    case 'archive':
      return 'archive'
    case 'restore':
      return 'restore'
    default:
      return 'update'
  }
})

const KnowledgeAcceptPolicySchema = z
  .unknown()
  .transform((value): 'auto_if_pass' | 'manual' | 'orchestrator_only' => {
    switch (String(value ?? '').trim()) {
      case 'auto_if_pass':
        return 'auto_if_pass'
      case 'orchestrator_only':
        return 'orchestrator_only'
      default:
        return 'manual'
    }
  })

const KnowledgeProposalItemSchema = z
  .object({
    item_id: z.string(),
    kind: z.preprocess(
      (value) => (typeof value === 'string' && value.trim() ? value : 'unknown'),
      z.string(),
    ),
    op: KnowledgeItemOpSchema,
    target_ref: z.string().optional(),
    target_revision: z.preprocess(
      (value) => {
        if (typeof value === 'number' && Number.isFinite(value)) return value
        if (typeof value === 'string') {
          const parsed = Number(value)
          return Number.isFinite(parsed) ? parsed : undefined
        }
        return undefined
      },
      z.number().optional(),
    ),
    fields: KnowledgeRecordSchema,
    evidence_refs: KnowledgeStringArraySchema,
    source_refs: KnowledgeStringArraySchema,
    change_reason: z.preprocess(
      (value) => (typeof value === 'string' ? value : value == null ? '' : String(value)),
      z.string(),
    ),
    accept_policy: KnowledgeAcceptPolicySchema,
  })
  .passthrough()

const KnowledgeProposalBundleSchema = z
  .object({
    schema_version: z.preprocess(
      (value) => (typeof value === 'number' && Number.isFinite(value) ? value : 1),
      z.number(),
    ),
    bundle_id: z.string(),
    scope_ref: z.preprocess(
      (value) => (typeof value === 'string' ? value : ''),
      z.string(),
    ),
    branch_id: z.string().optional(),
    source_session_id: z.preprocess(
      (value) => (typeof value === 'string' ? value : ''),
      z.string(),
    ),
    source_review_id: z.string().optional(),
    generated_at: z.preprocess(
      (value) => (typeof value === 'number' && Number.isFinite(value) ? value : 0),
      z.number(),
    ),
    proposal_items: z.preprocess(
      (value) => (Array.isArray(value) ? value : []),
      z.array(KnowledgeProposalItemSchema),
    ),
  })
  .passthrough()

const KnowledgeDeltaStatusSchema = z
  .unknown()
  .transform((value): 'proposed' | 'accepted' | 'applied' | 'rejected' => {
    switch (String(value ?? '').trim()) {
      case 'accepted':
        return 'accepted'
      case 'applied':
        return 'applied'
      case 'rejected':
        return 'rejected'
      default:
        return 'proposed'
    }
  })

const KnowledgeConflictSchema = z
  .object({
    type: z.preprocess(
      (value) => (typeof value === 'string' && value.trim() ? value : 'unknown'),
      z.string(),
    ),
    message: z.preprocess(
      (value) => (typeof value === 'string' ? value : value == null ? '' : String(value)),
      z.string(),
    ),
    item_id: z.string().optional(),
    target_ref: z.string().optional(),
  })
  .passthrough()

const KnowledgeDeltaTargetSchema = z
  .object({
    ref: z.preprocess(
      (value) => (typeof value === 'string' ? value : ''),
      z.string(),
    ),
    kind: z.preprocess(
      (value) => (typeof value === 'string' && value.trim() ? value : 'unknown'),
      z.string(),
    ),
    path: z.string().optional(),
  })
  .passthrough()

const KnowledgeDeltaChangeSchema = z
  .object({
    item_id: z.preprocess(
      (value) => (typeof value === 'string' ? value : ''),
      z.string(),
    ),
    op: z.preprocess(
      (value) => (typeof value === 'string' ? value : value == null ? '' : String(value)),
      z.string(),
    ),
    kind: z.preprocess(
      (value) => (typeof value === 'string' && value.trim() ? value : 'unknown'),
      z.string(),
    ),
    target_ref: z.string().optional(),
    summary: z.preprocess(
      (value) => (typeof value === 'string' ? value : value == null ? '' : String(value)),
      z.string(),
    ),
  })
  .passthrough()

const KnowledgeRollbackSchema = z
  .object({
    kind: z.preprocess(
      (value) => (String(value ?? '').trim() === 'hard' ? 'hard' : 'soft'),
      z.union([z.literal('soft'), z.literal('hard')]),
    ),
    token: z.string().optional(),
  })
  .passthrough()

const KnowledgeDeltaSchema = z
  .object({
    schema_version: z.preprocess(
      (value) => (typeof value === 'number' && Number.isFinite(value) ? value : 1),
      z.number(),
    ),
    knowledge_delta_id: z.string(),
    status: KnowledgeDeltaStatusSchema,
    scope_ref: z.preprocess(
      (value) => (typeof value === 'string' ? value : ''),
      z.string(),
    ),
    branch_id: z.string().optional(),
    source_session_id: z.preprocess(
      (value) => (typeof value === 'string' ? value : ''),
      z.string(),
    ),
    source_review_id: z.string().optional(),
    generated_at: z.preprocess(
      (value) => (typeof value === 'number' && Number.isFinite(value) ? value : 0),
      z.number(),
    ),
    targets: z.preprocess(
      (value) => (Array.isArray(value) ? value : []),
      z.array(KnowledgeDeltaTargetSchema),
    ),
    changes: z.preprocess(
      (value) => (Array.isArray(value) ? value : []),
      z.array(KnowledgeDeltaChangeSchema),
    ),
    evidence_refs: KnowledgeStringArraySchema,
    conflicts: z.preprocess(
      (value) => (Array.isArray(value) ? value : []),
      z.array(KnowledgeConflictSchema),
    ),
    accepted_item_ids: z.preprocess(
      (value) => (value === undefined ? undefined : Array.isArray(value) ? value.map((v) => String(v)) : []),
      z.array(z.string()).optional(),
    ),
    rejected_item_ids: z.preprocess(
      (value) => (value === undefined ? undefined : Array.isArray(value) ? value.map((v) => String(v)) : []),
      z.array(z.string()).optional(),
    ),
    applied_at: z.preprocess(
      (value) => (typeof value === 'number' && Number.isFinite(value) ? value : undefined),
      z.number().optional(),
    ),
    rollback: KnowledgeRollbackSchema.optional(),
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
  value = unwrapMaybeWrapped(value, 'result')
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
  const record = isRecord(resolved) ? resolved : isRecord(raw) ? raw : null
  if (!record) {
    return { bundle: null, delta: null }
  }

  const bundleCandidate = record.bundle
    ?? record.proposal_bundle
    ?? record.bundle_latest
    ?? record.latest_bundle
  const deltaCandidate = record.delta
    ?? record.knowledge_delta
    ?? record.delta_latest
    ?? record.latest_delta

  return {
    bundle: parseKnowledgeProposalBundle(bundleCandidate ?? null),
    delta: parseKnowledgeDelta(deltaCandidate ?? null),
  }
}

export async function missionKnowledgeDecide(
  projectPath: string,
  missionId: string,
  decision: KnowledgeDecisionInput,
): Promise<void> {
  await invoke<void>('mission_knowledge_decide', {
    input: { project_path: projectPath, mission_id: missionId, decision },
  })
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

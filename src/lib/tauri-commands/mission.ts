/**
 * mission.ts — Tauri command bindings for the Mission system.
 *
 * Mirrors the Rust commands in src-tauri/src/commands/mission.rs.
 */

import { invoke } from '@tauri-apps/api/core'
import { z } from 'zod'

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

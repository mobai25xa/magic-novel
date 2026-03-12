/**
 * macro-workflow.ts — M5 Macro Workflow types (v0 frozen contract).
 *
 * Aligned with worktrees/docs/M5/guide.md §2.2 / §2.3.
 * Compatibility policy: add-only fields, never delete.
 */

// ── MacroStage ──────────────────────────────────────────────────

export type MacroStage =
  | 'planning'
  | 'context'
  | 'draft'
  | 'review'
  | 'fix'
  | 'writeback'
  | 'integrate'
  | 'completed'
  | 'blocked'
  | 'failed'
  | 'cancelled'

// ── ChapterRunState ─────────────────────────────────────────────

export type ChapterRunStatus = 'pending' | 'running' | 'completed' | 'blocked' | 'failed' | 'skipped'

export type ChapterRunState = {
  chapter_ref: string
  write_path: string
  display_title?: string

  status: ChapterRunStatus
  stage?: MacroStage

  latest_contextpack_ref?: string
  latest_review_id?: string
  latest_knowledge_delta_id?: string

  last_handoff_summary?: string
  updated_at: number
} & Record<string, unknown>

// ── MacroWorkflowConfig (immutable input) ───────────────────────

export type MacroChapterTarget = {
  chapter_ref: string
  write_path: string
  display_title?: string
} & Record<string, unknown>

export type MacroTokenBudget = 'small' | 'medium' | 'large'

export type MacroWorkflowKind = 'book' | 'volume'

export type MacroWorkflowConfig = {
  schema_version: number
  macro_id: string
  mission_id: string
  workflow_kind: MacroWorkflowKind
  objective: string

  chapter_targets: MacroChapterTarget[]

  strict_review: boolean
  auto_fix_on_block: boolean
  token_budget: MacroTokenBudget

  created_at: number
} & Record<string, unknown>

// ── MacroWorkflowState (mutable progress) ───────────────────────

export type MacroLastError = {
  code: string
  message: string
  feature_id?: string
  worker_id?: string
} & Record<string, unknown>

export type MacroWorkflowState = {
  schema_version: number
  macro_id: string
  mission_id: string

  objective: string
  workflow_kind: MacroWorkflowKind

  current_index: number
  current_stage: MacroStage

  chapters: ChapterRunState[]

  last_transition_at: number
  last_error?: MacroLastError
} & Record<string, unknown>

// ── Tauri command I/O shapes ────────────────────────────────────

export type MacroCreateInput = {
  project_path: string
  objective: string
  workflow_kind: MacroWorkflowKind
  chapter_targets: MacroChapterTarget[]
  strict_review: boolean
  auto_fix_on_block: boolean
  token_budget: MacroTokenBudget
}

export type MacroCreateOutput = {
  mission_id: string
  macro_id: string
}

export type MacroGetStateOutput = {
  config: MacroWorkflowConfig | null
  state: MacroWorkflowState | null
}

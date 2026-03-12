/**
 * Macro workflow display types (dev-a).
 *
 * Aligned with guide.md v0 contract §2.2 / §2.3.
 * These are UI-only mirrors — canonical types live in dev-b's `src/types/`.
 */

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
}

export type MacroLastError = {
  code: string
  message: string
  feature_id?: string
  worker_id?: string
}

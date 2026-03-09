export interface ExportRequest {
  project_path: string
  chapter_path: string
  call_id: string
}

export interface ExportResult {
  chapter_id: string
  revision: number
  json_hash: string
  markdown: string
}

export type PreviewMode = 'replace' | 'patch-preferred'

export interface PreviewRequest {
  project_path: string
  chapter_path: string
  base_revision: number
  call_id: string
  mode: PreviewMode
  markdown: string
}

export type PatchOp =
  | { op: 'insert_blocks'; after_block_id?: string | null; blocks: unknown[] }
  | { op: 'update_block'; block_id: string; before?: unknown; after: unknown }
  | { op: 'delete_blocks'; block_ids: string[] }
  | { op: 'move_block'; block_id: string; after_block_id?: string | null }

export type DiagnosticLevel = 'info' | 'warn' | 'error'

export interface Diagnostic {
  level: DiagnosticLevel
  code: string
  message: string
  block_id?: string
  suggestion?: string
}

export interface PreviewResult {
  ok: boolean
  patch_ops: PatchOp[]
  diagnostics: Diagnostic[]
  diff_summary: string[]
  revision_before: number
}

export type Actor = 'agent' | 'user' | 'system'

export interface CommitRequest {
  project_path: string
  chapter_path: string
  base_revision: number
  call_id: string
  patch_ops: PatchOp[]
  actor: Actor
}

export interface CommitResult {
  ok: boolean
  revision_before: number
  revision_after: number
  json_hash_after: string
  tx_id: string
}

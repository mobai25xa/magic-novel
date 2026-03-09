import type { Actor, PreviewResult } from '../jvm-types'
import type { ToolLsInput, ToolLsOutput } from './ls-types'

export type FaultDomain =
  | 'tool'
  | 'validation'
  | 'policy'
  | 'jvm'
  | 'vc'
  | 'io'
  | 'network'
  | 'auth'
  | 'external'

export type ToolName =
  | 'create'
  | 'read'
  | 'edit'
  | 'delete'
  | 'move'
  | 'ls'
  | 'grep'
  | 'askuser'
  | 'skill'
  | 'todowrite'
  | 'outline'
  | 'character_sheet'
  | 'search_knowledge'

export interface ToolError {
  code: string
  message: string
  retryable: boolean
  fault_domain: FaultDomain
}

export interface ToolMeta {
  tool: ToolName
  call_id: string
  duration_ms: number
  revision_before?: number
  revision_after?: number
  tx_id?: string
  read_set?: string[]
  write_set?: string[]
}

export interface ToolResult<T> {
  ok: boolean
  data?: T
  error?: ToolError
  meta: ToolMeta
}

export type ToolCreateKind = 'volume' | 'chapter'

interface ToolCreateBaseInput {
  project_path: string
  title: string
  call_id?: string
}

export type ToolCreateInput =
  | (ToolCreateBaseInput & {
      kind: 'volume'
      volume_path?: never
    })
  | (ToolCreateBaseInput & {
      kind: 'chapter'
      volume_path: string
    })

export interface ToolCreateOutput {
  created_kind: ToolCreateKind
  path: string
  id: string
  revision_after: number
  created_at: number
  chapter_id?: string
  chapter_path?: string
  revision?: number
}

export type ToolReadKind = 'volume' | 'chapter'
export type ToolReadView = 'meta' | 'snapshot' | 'json'

interface ToolReadBaseInput {
  project_path: string
  path: string
  call_id?: string
}

export type ToolReadInput =
  | (ToolReadBaseInput & {
      kind: 'volume'
      view: 'meta'
    })
  | (ToolReadBaseInput & {
      kind: 'chapter'
      view: ToolReadView
    })

export interface ToolSnapshotBlock {
  block_id: string
  block_type: string
  order: number
  markdown: string
}

export interface ToolChapterSnapshot {
  snapshot_id: string
  block_count: number
  blocks: ToolSnapshotBlock[]
}

export interface ToolReadOutput {
  path: string
  kind: ToolReadKind
  revision: number
  hash: string
  metadata: Record<string, unknown>
  snapshot?: ToolChapterSnapshot
  content_json?: unknown
  chapter_id?: string
  json_hash?: string
}

export type ToolEditTarget = 'volume_meta' | 'chapter_meta' | 'chapter_content'

interface ToolEditBaseInput {
  project_path: string
  path: string
  call_id?: string
  dry_run?: boolean
}

export interface ToolEditVolumeMetaInput extends ToolEditBaseInput {
  target: 'volume_meta'
  title?: string
  summary?: string
}

export interface ToolEditChapterMetaInput extends ToolEditBaseInput {
  target: 'chapter_meta'
  title?: string
  summary?: string
  status?: 'draft' | 'revised' | 'final'
  target_words?: number
  tags?: string[]
  pinned_assets?: unknown[]
}

export interface ToolEditBlockInput {
  markdown: string
}

export type ToolEditOp =
  | {
      op: 'replace_block'
      block_id: string
      markdown: string
    }
  | {
      op: 'delete_block'
      block_id: string
    }
  | {
      op: 'insert_before'
      block_id: string
      blocks: ToolEditBlockInput[]
    }
  | {
      op: 'insert_after'
      block_id: string
      blocks: ToolEditBlockInput[]
    }
  | {
      op: 'append_blocks'
      blocks: ToolEditBlockInput[]
    }
  | {
      op: 'replace_range'
      start_block_id: string
      end_block_id: string
      blocks: ToolEditBlockInput[]
    }

export interface ToolEditChapterContentInputV2 extends ToolEditBaseInput {
  target: 'chapter_content'
  base_revision: number
  snapshot_id: string
  ops: ToolEditOp[]
  actor?: Actor
}

export type ToolEditInput =
  | ToolEditVolumeMetaInput
  | ToolEditChapterMetaInput
  | ToolEditChapterContentInputV2

export interface ToolEditOutput {
  mode: 'preview' | 'commit'
  accepted: boolean
  path: string
  target: ToolEditTarget
  revision_before?: number
  revision_after?: number
  diagnostics: PreviewResult['diagnostics']
  changed_fields?: string[]
  changed_block_ids?: string[]
  snapshot_id?: string
  diff_summary: string[]
  tx_id?: string
  hash_after?: string
  json_hash_after?: string
}

export interface ToolDeleteInput {
  project_path: string
  kind: 'volume' | 'chapter'
  path: string
  dry_run?: boolean
  call_id?: string
}

export interface ToolDeleteOutput {
  mode: 'preview' | 'commit'
  kind: 'volume' | 'chapter'
  path: string
  accepted?: boolean
  impact: Record<string, unknown>
  tx_id?: string
}

export interface ToolMoveInput {
  project_path: string
  chapter_path: string
  target_volume_path: string
  target_index: number
  dry_run?: boolean
  call_id?: string
}

export interface ToolMoveOutput {
  mode: 'preview' | 'commit'
  accepted: boolean
  chapter_path: string
  target_volume_path: string
  target_index: number
  new_chapter_path?: string
  impact?: Record<string, unknown>
  tx_id?: string
}

export interface ToolGrepInput {
  project_path: string
  query: string
  mode?: 'keyword' | 'semantic' | 'hybrid'
  scope?: { paths?: string[] }
  top_k?: number
  call_id?: string
}

export interface ToolGrepOutput {
  hits: Array<{
    path: string
    score: number
    snippet: string
    metadata?: Record<string, unknown>
  }>
  semantic_notice?: {
    semantic_retrieval_available: boolean
    reason?: string
    message?: string
  }
}

export interface ToolGateway {
  create(input: ToolCreateInput): Promise<ToolResult<ToolCreateOutput>>
  read(input: ToolReadInput): Promise<ToolResult<ToolReadOutput>>
  edit(input: ToolEditInput): Promise<ToolResult<ToolEditOutput>>
  delete(input: ToolDeleteInput): Promise<ToolResult<ToolDeleteOutput>>
  move(input: ToolMoveInput): Promise<ToolResult<ToolMoveOutput>>
  ls(input: ToolLsInput): Promise<ToolResult<ToolLsOutput>>
  grep(input: ToolGrepInput): Promise<ToolResult<ToolGrepOutput>>
}

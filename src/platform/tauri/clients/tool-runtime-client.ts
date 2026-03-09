import type {
  ToolCreateInput,
  ToolCreateOutput,
  ToolDeleteInput,
  ToolDeleteOutput,
  ToolEditInput,
  ToolEditOutput,
  ToolGrepInput,
  ToolGrepOutput,
  ToolMoveInput,
  ToolMoveOutput,
  ToolReadInput,
  ToolReadOutput,
  ToolResult,
} from '@/lib/tool-gateway/types'
import type { ToolLsInput, ToolLsOutput } from '@/lib/tool-gateway/ls-types'

import { invokeTauri } from './core'

type AnyRecord = Record<string, unknown>

function ensureCallId(callId?: string): string | undefined {
  return callId?.trim() ? callId : undefined
}

export async function runtimeToolCreate(input: ToolCreateInput): Promise<ToolResult<ToolCreateOutput>> {
  const isChapter = input.kind === 'chapter'

  const payload = {
    input: {
      project_path: input.project_path,
      kind: input.kind,
      title: input.title,
      volume_path: isChapter ? input.volume_path : undefined,
      cwd: isChapter ? input.volume_path : '.',
      node_kind: isChapter ? 'file' : 'folder',
      name: input.title,
      content: '',
      content_format: 'text',
      metadata: {},
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_create', normalizeCallIdPayload(payload))
  return mapCreateResult(res)
}

export async function runtimeToolRead(input: ToolReadInput): Promise<ToolResult<ToolReadOutput>> {
  const payload = {
    input: {
      project_path: input.project_path,
      kind: input.kind,
      path: input.path,
      view: input.view,
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_read', normalizeCallIdPayload(payload))
  return mapReadResult(res)
}

export async function runtimeToolEdit(input: ToolEditInput): Promise<ToolResult<ToolEditOutput>> {
  const payload = {
    input: {
      project_path: input.project_path,
      path: input.path,
      target: input.target,
      dry_run: input.dry_run ?? false,
      title: 'title' in input ? input.title : undefined,
      summary: 'summary' in input ? input.summary : undefined,
      status: 'status' in input ? input.status : undefined,
      target_words: 'target_words' in input ? input.target_words : undefined,
      tags: 'tags' in input ? input.tags : undefined,
      pinned_assets: 'pinned_assets' in input ? input.pinned_assets : undefined,
      base_revision: input.target === 'chapter_content' ? input.base_revision : 0,
      snapshot_id: input.target === 'chapter_content' ? input.snapshot_id : undefined,
      ops: input.target === 'chapter_content' ? input.ops : undefined,
      actor: input.target === 'chapter_content' ? (input.actor || 'agent') : 'agent',
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_edit', normalizeCallIdPayload(payload))
  return mapEditResult(res)
}

export async function runtimeToolDelete(input: ToolDeleteInput): Promise<ToolResult<ToolDeleteOutput>> {
  const payload = {
    input: {
      project_path: input.project_path,
      kind: input.kind,
      path: input.path,
      dry_run: input.dry_run ?? false,
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_delete', normalizeCallIdPayload(payload))
  return mapDeleteResult(res)
}

export async function runtimeToolMove(input: ToolMoveInput): Promise<ToolResult<ToolMoveOutput>> {
  const payload = {
    input: {
      project_path: input.project_path,
      chapter_path: input.chapter_path,
      target_volume_path: input.target_volume_path,
      target_index: input.target_index,
      dry_run: input.dry_run ?? false,
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_move', normalizeCallIdPayload(payload))
  return mapMoveResult(res)
}

export async function runtimeToolLs(input: ToolLsInput): Promise<ToolResult<ToolLsOutput>> {
  const offset = Number.isFinite(input.offset) ? Math.max(0, Math.floor(input.offset as number)) : 0
  const limitRaw = Number.isFinite(input.limit) ? Math.floor(input.limit as number) : 30
  const limit = Math.min(200, Math.max(1, limitRaw))

  const payload = {
    input: {
      project_path: input.project_path,
      cwd: input.path || '.',
      depth: offset + limit,
      include_hidden: false,
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_ls', normalizeCallIdPayload(payload))
  return mapLsResult(res)
}

export async function runtimeToolGrep(input: ToolGrepInput): Promise<ToolResult<ToolGrepOutput>> {
  const payload = {
    input: {
      project_path: input.project_path,
      query: input.query,
      mode: input.mode || 'keyword',
      scope: input.scope ? { paths: input.scope.paths || [] } : undefined,
      top_k: input.top_k ?? 10,
    },
    call_id: ensureCallId(input.call_id),
  }

  const res = await invokeTauri<ToolResult<AnyRecord>>('tool_grep', normalizeCallIdPayload(payload))
  return mapGrepResult(res)
}

function mapCreateResult(res: ToolResult<AnyRecord>): ToolResult<ToolCreateOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  const path = asString(res.data.path) || asString(res.data.chapter_path)
  const id = asString(res.data.id) || asString(res.data.chapter_id)
  const created_kind = resolveCreateKind(res.data.created_kind ?? res.data.kind, path)
  const revision_after = asNumber(res.data.revision_after) || asNumber(res.data.revision)
  const chapterPath = created_kind === 'chapter' ? path : undefined
  const chapterId = created_kind === 'chapter' ? id : undefined

  return {
    ...res,
    data: {
      created_kind,
      path,
      id,
      revision_after,
      created_at: asNumber(res.data.created_at),
      chapter_id: chapterId,
      chapter_path: chapterPath,
      revision: chapterPath ? revision_after : undefined,
    },
  }
}

function mapReadResult(res: ToolResult<AnyRecord>): ToolResult<ToolReadOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  const metadata = asRecord(res.data.metadata)
  const path = asString(res.data.path)
  const kind = resolveReadKind(res.data.kind, metadata)
  const hash = asString(res.data.hash) || asString(res.data.json_hash)
  const chapterId = asOptionalString(metadata.chapter_id) ?? asOptionalString(res.data.chapter_id)
  const snapshotRaw = asRecord(res.data.snapshot)
  const blocks = Array.isArray(snapshotRaw?.blocks)
    ? snapshotRaw.blocks
      .map((row) => asRecord(row))
      .filter((row): row is AnyRecord => Boolean(row))
      .map((row, index) => ({
        block_id: asString(row.block_id),
        block_type: asString(row.block_type),
        order: asNumber(row.order) || index,
        markdown: asString(row.markdown),
      }))
    : []
  const snapshot = snapshotRaw
    ? {
      snapshot_id: asString(snapshotRaw.snapshot_id),
      block_count: asNumber(snapshotRaw.block_count) || blocks.length,
      blocks,
    }
    : undefined

  return {
    ...res,
    data: {
      path,
      kind,
      revision: asNumber(res.data.revision),
      hash,
      metadata,
      snapshot,
      content_json: res.data.content_json,
      chapter_id: chapterId,
      json_hash: hash || undefined,
    },
  }
}

function mapEditResult(res: ToolResult<AnyRecord>): ToolResult<ToolEditOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  const target = resolveEditTarget(res.data.target)
  const hashAfter = asOptionalString(res.data.hash_after) ?? asOptionalString(res.data.json_hash_after)

  return {
    ...res,
    data: {
      mode: asString(res.data.mode) === 'commit' ? 'commit' : 'preview',
      accepted: Boolean(res.data.accepted),
      path: asString(res.data.path),
      target,
      revision_before: asOptionalNumber(res.data.revision_before),
      revision_after: asOptionalNumber(res.data.revision_after),
      diagnostics: Array.isArray(res.data.diagnostics)
        ? (res.data.diagnostics as ToolEditOutput['diagnostics'])
        : [],
      changed_fields: parseStringList(res.data.changed_fields),
      changed_block_ids: parseStringList(res.data.changed_block_ids),
      snapshot_id: asOptionalString(res.data.snapshot_id),
      diff_summary: parseDiffSummary(res.data.diff_summary),
      tx_id: asOptionalString(res.data.tx_id),
      hash_after: hashAfter,
      json_hash_after: hashAfter,
    },
  }
}

function mapDeleteResult(res: ToolResult<AnyRecord>): ToolResult<ToolDeleteOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  return {
    ...res,
    data: {
      mode: asString(res.data.mode) === 'commit' ? 'commit' : 'preview',
      kind: resolveDeleteKind(res.data.kind),
      path: asString(res.data.path),
      accepted: asOptionalBoolean(res.data.accepted),
      impact: asRecord(res.data.impact),
      tx_id: asOptionalString(res.data.tx_id),
    },
  }
}

function mapMoveResult(res: ToolResult<AnyRecord>): ToolResult<ToolMoveOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  return {
    ...res,
    data: {
      mode: asString(res.data.mode) === 'commit' ? 'commit' : 'preview',
      accepted: Boolean(res.data.accepted),
      chapter_path: asString(res.data.chapter_path),
      target_volume_path: asString(res.data.target_volume_path),
      target_index: asNumber(res.data.target_index),
      new_chapter_path: asOptionalString(res.data.new_chapter_path),
      impact: Object.keys(asRecord(res.data.impact)).length ? asRecord(res.data.impact) : undefined,
      tx_id: asOptionalString(res.data.tx_id),
    },
  }
}

function mapLsResult(res: ToolResult<AnyRecord>): ToolResult<ToolLsOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  const itemsRaw = Array.isArray(res.data.items) ? res.data.items : []
  const items = itemsRaw.map((raw) => {
    const item = asRecord(raw)
    const metadata = asRecord(item.metadata)
    const kind = resolveLsKind(metadata.type)

    return {
      kind,
      name: asString(item.name),
      path: asString(item.path),
      title: asOptionalString(item.name),
      child_count: asOptionalNumber(item.child_count),
      chapter_id: asOptionalString(metadata.chapter_id),
    }
  })

  return {
    ...res,
    data: {
      cwd: asString(res.data.cwd) || '.',
      items,
    },
  }
}

function mapGrepResult(res: ToolResult<AnyRecord>): ToolResult<ToolGrepOutput> {
  if (!res.ok || !res.data) return { ...res, data: undefined }

  const hitsRaw = Array.isArray(res.data.hits) ? res.data.hits : []
  const hits = hitsRaw.map((raw) => {
    const hit = asRecord(raw)
    return {
      path: asString(hit.path),
      score: asNumber(hit.score),
      snippet: asString(hit.snippet),
      metadata: asRecord(hit.metadata),
    }
  })

  const semanticNotice = parseSemanticNotice(res.data.semantic_notice)

  return {
    ...res,
    data: {
      hits,
      semantic_notice: semanticNotice,
    },
  }
}

function parseSemanticNotice(input: unknown): ToolGrepOutput['semantic_notice'] | undefined {
  const record = asRecord(input)
  if (Object.keys(record).length === 0) return undefined

  return {
    semantic_retrieval_available: Boolean(record.semantic_retrieval_available),
    reason: asOptionalString(record.reason),
    message: asOptionalString(record.message),
  }
}

function resolveCreateKind(input: unknown, path: string): ToolCreateOutput['created_kind'] {
  const kind = asString(input).trim().toLowerCase()
  if (kind === 'volume' || kind === 'folder') return 'volume'
  if (kind === 'chapter' || kind === 'file') return 'chapter'
  return path.endsWith('.json') ? 'chapter' : 'volume'
}

function resolveReadKind(input: unknown, metadata: AnyRecord): ToolReadOutput['kind'] {
  const kind = asString(input).trim().toLowerCase()
  const metadataType = asString(metadata.type).trim().toLowerCase()

  if (kind === 'volume' || kind === 'folder') return 'volume'
  if (kind === 'chapter' || kind === 'file') return 'chapter'
  if (metadataType === 'volume') return 'volume'
  return 'chapter'
}

function resolveEditTarget(input: unknown): ToolEditOutput['target'] {
  const target = asString(input).trim().toLowerCase()
  if (target === 'volume_meta') return 'volume_meta'
  if (target === 'chapter_meta') return 'chapter_meta'
  return 'chapter_content'
}

function resolveDeleteKind(input: unknown): ToolDeleteOutput['kind'] {
  const kind = asString(input).trim().toLowerCase()
  return kind === 'volume' ? 'volume' : 'chapter'
}

function parseDiffSummary(input: unknown): string[] {
  if (!Array.isArray(input)) return []

  return input
    .map((item) => {
      if (typeof item === 'string') return item

      const entry = asRecord(item)
      const operation = asOptionalString(entry.operation)
      const description = asOptionalString(entry.description)
      if (operation && description) return `${operation}: ${description}`
      if (description) return description
      if (operation) return operation
      return JSON.stringify(entry)
    })
    .filter((item): item is string => Boolean(item))
}

function parseStringList(input: unknown): string[] | undefined {
  if (!Array.isArray(input)) return undefined
  const values = input.map((value) => String(value)).filter(Boolean)
  return values.length ? values : undefined
}

function normalizeCallIdPayload<T extends { call_id?: string }>(payload: T): Omit<T, 'call_id'> & { callId?: string } {
  const next = { ...payload } as T & { callId?: string }
  if (typeof payload.call_id === 'string' && payload.call_id.trim()) {
    next.callId = payload.call_id
  }

  delete (next as { call_id?: string }).call_id
  return next as Omit<T, 'call_id'> & { callId?: string }
}

function resolveLsKind(input: unknown): ToolLsOutput['items'][number]['kind'] {
  const type = String(input || '')
  if (type === 'chapter') return 'chapter'
  if (type === 'knowledge_root') return 'knowledge_root'
  if (type === 'knowledge_folder') return 'knowledge_folder'
  if (type === 'knowledge_file') return 'knowledge_file'
  return 'volume'
}

function asRecord(input: unknown): AnyRecord {
  if (input && typeof input === 'object') return input as AnyRecord
  return {}
}

function asString(input: unknown): string {
  if (typeof input === 'string') return input
  if (typeof input === 'number' || typeof input === 'boolean') return String(input)
  return ''
}

function asOptionalString(input: unknown): string | undefined {
  return typeof input === 'string' ? input : undefined
}

function asNumber(input: unknown): number {
  return typeof input === 'number' && Number.isFinite(input) ? input : 0
}

function asOptionalNumber(input: unknown): number | undefined {
  return typeof input === 'number' && Number.isFinite(input) ? input : undefined
}

function asOptionalBoolean(input: unknown): boolean | undefined {
  return typeof input === 'boolean' ? input : undefined
}

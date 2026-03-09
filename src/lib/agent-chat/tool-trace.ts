import type {
  AgentPendingAskUserRequest,
  AgentTodoItem,
  AgentTodoState,
  AgentTodoStatus,
} from '@/agent/types'
import type { FaultDomain } from '@/lib/tool-gateway/types'

import {
  mapStructuredAskUserQuestions,
  parseAskUserQuestionnaire,
} from './askuser'

const MAX_TODO_ITEMS = 50
const MAX_TODO_TEXT_LENGTH = 500

export type ToolTraceStage = 'policy' | 'execute' | 'result'

export interface ToolTraceV2 {
  schema_version: 2
  stage: ToolTraceStage
  meta: {
    tool: string
    call_id: string
    duration_ms: number
    revision_before?: number
    revision_after?: number
    tx_id?: string
  }
  result: {
    ok: boolean
    preview: Record<string, unknown>
    error: Record<string, unknown> | null
  }
  refs?: {
    path?: string
    entity_id?: string
    snapshot_id?: string
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

function toMaybeString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined
  }
  const text = value.trim()
  return text || undefined
}

function toMaybeNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function normalizeStage(value: unknown): ToolTraceStage | null {
  if (value === 'policy' || value === 'execute' || value === 'result') {
    return value
  }
  return null
}

function normalizeTodoStatus(value: unknown): AgentTodoStatus | null {
  const status = toMaybeString(value)?.toLowerCase()
  if (status === 'pending' || status === 'in_progress' || status === 'completed') {
    return status
  }
  return null
}

function parseTodoItems(raw: unknown): AgentTodoItem[] | null {
  if (!Array.isArray(raw) || raw.length === 0 || raw.length > MAX_TODO_ITEMS) {
    return null
  }
  const items: AgentTodoItem[] = []
  let inProgressSeen = false
  for (const row of raw) {
    const entry = asRecord(row)
    if (!entry) {
      return null
    }
    const status = normalizeTodoStatus(entry.status)
    const text = toMaybeString(entry.text)
    if (!status || !text || text.length > MAX_TODO_TEXT_LENGTH) {
      return null
    }
    if (status === 'in_progress') {
      if (inProgressSeen) {
        return null
      }
      inProgressSeen = true
    }
    items.push({ status, text })
  }
  return items
}

export function parseToolTraceV2(trace: unknown): ToolTraceV2 | null {
  const root = asRecord(trace)
  if (!root || root.schema_version !== 2) {
    return null
  }
  const stage = normalizeStage(root.stage)
  const meta = asRecord(root.meta)
  const result = asRecord(root.result)
  const refs = asRecord(root.refs)
  if (!stage || !meta || !result) {
    return null
  }
  const tool = toMaybeString(meta.tool)
  const callId = toMaybeString(meta.call_id)
  const durationMs = toMaybeNumber(meta.duration_ms)
  const ok = result.ok === true || result.ok === false ? result.ok : null
  const preview = asRecord(result.preview) || {}
  const error = asRecord(result.error)
  if (!tool || !callId || typeof durationMs !== 'number' || ok === null) {
    return null
  }

  return {
    schema_version: 2,
    stage,
    meta: {
      tool,
      call_id: callId,
      duration_ms: durationMs,
      revision_before: toMaybeNumber(meta.revision_before),
      revision_after: toMaybeNumber(meta.revision_after),
      tx_id: toMaybeString(meta.tx_id),
    },
    result: {
      ok,
      preview,
      error,
    },
    refs: refs
      ? {
          path: toMaybeString(refs.path),
          entity_id: toMaybeString(refs.entity_id),
          snapshot_id: toMaybeString(refs.snapshot_id),
        }
      : undefined,
  }
}

export function extractTodoStateFromTrace(input: {
  trace: unknown
  fallbackCallId?: string
}): AgentTodoState | null {
  const parsed = parseToolTraceV2(input.trace)
  if (parsed && parsed.meta.tool === 'todowrite' && parsed.result.ok) {
    const todoStateRaw = asRecord(parsed.result.preview.todo_state)
    if (!todoStateRaw) {
      return null
    }
    const items = parseTodoItems(todoStateRaw.items)
    if (!items) {
      return null
    }
    const lastUpdatedAt = toMaybeNumber(todoStateRaw.last_updated_at) ?? Date.now()
    return {
      items,
      lastUpdatedAt,
      sourceCallId: toMaybeString(todoStateRaw.source_call_id)
        || toMaybeString(input.fallbackCallId)
        || parsed.meta.call_id,
    }
  }

  const legacyTrace = asRecord(input.trace)
  const legacyResult = asRecord(legacyTrace?.result)
  const legacyData = asRecord(legacyResult?.data)
  const todoStateRaw = asRecord(legacyData?.todo_state)
  if (!todoStateRaw) {
    return null
  }
  const items = parseTodoItems(todoStateRaw.items)
  if (!items) {
    return null
  }
  const lastUpdatedAt = toMaybeNumber(todoStateRaw.last_updated_at) ?? Date.now()
  return {
    items,
    lastUpdatedAt,
    sourceCallId: toMaybeString(todoStateRaw.source_call_id)
      || toMaybeString(input.fallbackCallId)
      || toMaybeString(legacyTrace?.call_id),
  }
}

export function extractAskUserFromTrace(input: {
  trace: unknown
  turn: number
  requestedAt: number
}): AgentPendingAskUserRequest | null {
  const parsed = parseToolTraceV2(input.trace)
  if (!parsed || parsed.meta.tool !== 'askuser') {
    return null
  }
  const preview = parsed.result.preview
  const askUser = asRecord(preview.askuser_request) || preview
  const callId = toMaybeString(askUser.call_id) || parsed.meta.call_id
  if (!callId) {
    return null
  }
  let questions = mapStructuredAskUserQuestions(askUser.questions)
  let questionnaire = toMaybeString(askUser.questionnaire) || ''
  if ((!questions || questions.length === 0) && questionnaire) {
    const parsedQuestionnaire = parseAskUserQuestionnaire(questionnaire)
    if (parsedQuestionnaire.ok) {
      questions = parsedQuestionnaire.questions
      questionnaire = parsedQuestionnaire.questionnaire
    }
  }
  if (!questions || questions.length === 0) {
    return null
  }
  if (!questionnaire) {
    questionnaire = questions.map((question, index) => `${index + 1}. ${question.question}`).join('\n')
  }

  return {
    callId,
    turn: input.turn,
    questionnaire,
    questions,
    requestedAt: input.requestedAt,
  }
}

export function extractToolPreviewRefs(trace: unknown): {
  path?: string
  entity_id?: string
  snapshot_id?: string
  changed_block_ids?: string[]
} | null {
  const parsed = parseToolTraceV2(trace)
  if (!parsed) {
    return null
  }
  const preview = parsed.result.preview
  const changedBlockIds = Array.isArray(preview.changed_block_ids)
    ? preview.changed_block_ids
      .filter((value): value is string => typeof value === 'string' && value.trim().length > 0)
    : []
  const path = parsed.refs?.path
    || toMaybeString(preview.path)
    || toMaybeString(preview.chapter_path)
    || toMaybeString(preview.new_chapter_path)
  const entityId = parsed.refs?.entity_id
  const snapshotId = parsed.refs?.snapshot_id || toMaybeString(preview.snapshot_id)
  if (!path && !entityId && !snapshotId && changedBlockIds.length === 0) {
    return null
  }

  return {
    path,
    entity_id: entityId,
    snapshot_id: snapshotId,
    changed_block_ids: changedBlockIds.length > 0 ? changedBlockIds : undefined,
  }
}

export function toFaultDomain(value: unknown): FaultDomain | undefined {
  if (
    value === 'tool'
    || value === 'validation'
    || value === 'policy'
    || value === 'jvm'
    || value === 'vc'
    || value === 'io'
    || value === 'network'
    || value === 'auth'
    || value === 'external'
  ) {
    return value
  }
  return undefined
}

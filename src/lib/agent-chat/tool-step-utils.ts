import type { AgentToolTrace } from '@/agent/types'

const MAX_RAW_OUTPUT_CHARS = 4000
const SENSITIVE_KEY_RE = /(api[-_]?key|token|authorization|cookie|secret|password)/i

type AnyRecord = Record<string, unknown>

type ToolStepOutput = {
  parsedOutput: unknown
  outputPreview: unknown
  rawOutput: string
  retryable?: boolean
  errorMessage?: string
  resultSummary?: string
}

function asRecord(value: unknown): AnyRecord | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as AnyRecord
}

export function safeParseJson(value: string): unknown {
  try {
    return JSON.parse(value)
  } catch {
    return value
  }
}

export function trimRawOutput(output: string) {
  if (output.length <= MAX_RAW_OUTPUT_CHARS) {
    return output
  }
  return `${output.slice(0, MAX_RAW_OUTPUT_CHARS)}…[TRUNCATED]`
}

export function redactValue(value: unknown, depth = 0): unknown {
  if (depth > 6) {
    return '[MAX_DEPTH]'
  }

  if (Array.isArray(value)) {
    if (value.length > 20) {
      return [...value.slice(0, 20).map((item) => redactValue(item, depth + 1)), `[+${value.length - 20} items]`]
    }
    return value.map((item) => redactValue(item, depth + 1))
  }

  if (value && typeof value === 'object') {
    const result: AnyRecord = {}
    for (const [key, nested] of Object.entries(value as AnyRecord)) {
      result[key] = SENSITIVE_KEY_RE.test(key) ? '[REDACTED]' : redactValue(nested, depth + 1)
    }
    return result
  }

  if (typeof value === 'string' && value.length > 500) {
    return `${value.slice(0, 500)}…`
  }

  return value
}

function toShort(value: unknown) {
  if (typeof value === 'string') {
    return value.length > 40 ? `${value.slice(0, 40)}…` : value
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value)
  }
  return '[object]'
}

function joinSummary(prefix: string, parts: Array<string | null | undefined>) {
  const clean = parts.filter((part): part is string => Boolean(part))
  return clean.length ? `${prefix} · ${clean.join(', ')}` : prefix
}

export function summarizeArgs(toolName: string, args: Record<string, unknown>) {
  const keysByTool: Record<string, string[]> = {
    read: ['kind', 'path', 'view'],
    create: ['kind', 'title', 'volume_path'],
    edit: ['target', 'path', 'base_revision', 'snapshot_id', 'dry_run'],
    delete: ['kind', 'path', 'dry_run'],
    move: ['chapter_path', 'target_volume_path', 'target_index', 'dry_run'],
    ls: ['path', 'cwd', 'depth'],
    grep: ['query', 'mode', 'top_k'],
    todowrite: ['todos'],
  }
  const keys = keysByTool[toolName] || Object.keys(args)

  const segments: string[] = []
  for (const key of keys) {
    if (!(key in args)) continue
    const value = args[key]
    if (value === undefined || value === null || value === '') continue
    segments.push(`${key}=${toShort(value)}`)
    if (segments.length >= 4) break
  }

  if (!segments.length) {
    return 'no args'
  }
  return segments.join(', ')
}

export function extractErrorMessage(parsedOutput: unknown, trace: { fault_domain?: string; error_code?: string; error_message?: string }) {
  const parsedRecord = asRecord(parsedOutput)
  const error = asRecord(parsedRecord?.error)
  if (typeof error?.message === 'string' && error.message.trim()) {
    return error.message
  }

  if (typeof trace.error_message === 'string' && trace.error_message.trim()) {
    return trace.error_message
  }

  if (trace.error_code) {
    return `${trace.fault_domain || 'tool'}:${trace.error_code}`
  }

  return undefined
}

export function extractRetryable(parsedOutput: unknown) {
  const parsedRecord = asRecord(parsedOutput)
  const error = asRecord(parsedRecord?.error)
  return typeof error?.retryable === 'boolean' ? error.retryable : undefined
}

export function summarizeResult(toolName: string, parsedOutput: unknown, trace: { status: 'ok' | 'error'; stage?: AgentToolTrace['stage']; error_code?: string; fault_domain?: AgentToolTrace['fault_domain'] }) {
  const parsedRecord = asRecord(parsedOutput)
  const ok = parsedRecord?.ok === true

  if (!parsedRecord) {
    return joinSummary(`${toolName} ${trace.status}`, [trace.error_code])
  }

  if (ok) {
    const data = asRecord(parsedRecord.data)
    if (toolName === 'read') {
      const content = typeof data?.content === 'string'
        ? `chars=${data.content.length}`
        : (typeof data?.markdown === 'string' ? `chars=${data.markdown.length}` : null)
      const revision = typeof data?.revision === 'number' ? `rev=${data.revision}` : null
      const path = typeof data?.path === 'string' ? data.path : null
      return joinSummary('read ok', [path, revision, content])
    }
    if (toolName === 'create') {
      const path = typeof data?.path === 'string'
        ? data.path
        : (typeof data?.chapter_path === 'string' ? data.chapter_path : null)
      const createdKind = typeof data?.created_kind === 'string' ? data.created_kind : null
      const revision = typeof data?.revision_after === 'number'
        ? `rev=${data.revision_after}`
        : (typeof data?.revision === 'number' ? `rev=${data.revision}` : null)
      return joinSummary('create ok', [createdKind, path, revision])
    }
    if (toolName === 'edit') {
      const mode = typeof data?.mode === 'string' ? `mode=${data.mode}` : null
      const accepted = typeof data?.accepted === 'boolean' ? `accepted=${String(data.accepted)}` : null
      const target = typeof data?.target === 'string' ? `target=${data.target}` : null
      const path = typeof data?.path === 'string' ? data.path : null
      const revision = typeof data?.revision_after === 'number' ? `rev=${data.revision_after}` : null
      const changedFields = Array.isArray(data?.changed_fields) ? `fields=${data.changed_fields.length}` : null
      return joinSummary('edit ok', [path, target, mode, accepted, changedFields, revision])
    }
    if (toolName === 'delete') {
      const mode = typeof data?.mode === 'string' ? `mode=${data.mode}` : null
      const kind = typeof data?.kind === 'string' ? `kind=${data.kind}` : null
      const path = typeof data?.path === 'string' ? data.path : null
      return joinSummary('delete ok', [mode, kind, path])
    }
    if (toolName === 'move') {
      const mode = typeof data?.mode === 'string' ? `mode=${data.mode}` : null
      const accepted = typeof data?.accepted === 'boolean' ? `accepted=${String(data.accepted)}` : null
      const chapterPath = typeof data?.chapter_path === 'string' ? data.chapter_path : null
      const newPath = typeof data?.new_chapter_path === 'string' ? data.new_chapter_path : null
      return joinSummary('move ok', [mode, accepted, chapterPath, newPath])
    }
    if (toolName === 'ls') {
      const items = Array.isArray(data?.items) ? `items=${data.items.length}` : null
      const cwd = typeof data?.cwd === 'string' ? data.cwd : null
      return joinSummary('ls ok', [cwd, items])
    }
    if (toolName === 'grep') {
      const hits = Array.isArray(data?.hits) ? `hits=${data.hits.length}` : null
      return joinSummary('grep ok', [hits])
    }
    if (toolName === 'todowrite') {
      return joinSummary('todowrite ok', ['updated=true'])
    }
    return joinSummary(`${toolName} ok`, [trace.stage || 'result'])
  }

  const message = extractErrorMessage(parsedOutput, trace)
  if (message) {
    return message
  }
  return joinSummary(`${toolName} error`, [trace.fault_domain, trace.error_code])
}

export function buildToolStepOutput(input: {
  toolName: string
  output: string
  trace: {
    status: 'ok' | 'error'
    stage?: AgentToolTrace['stage']
    fault_domain?: AgentToolTrace['fault_domain']
    error_code?: string
    error_message?: string
  }
}): ToolStepOutput {
  const parsedOutput = safeParseJson(input.output)
  const resultSummary = summarizeResult(input.toolName, parsedOutput, input.trace)

  return {
    parsedOutput,
    outputPreview: redactValue(parsedOutput),
    rawOutput: trimRawOutput(input.output),
    retryable: extractRetryable(parsedOutput),
    errorMessage: extractErrorMessage(parsedOutput, input.trace),
    resultSummary,
  }
}

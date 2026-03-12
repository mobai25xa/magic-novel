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

function normalizeIssueSeverity(value: unknown): 'info' | 'warn' | 'block' | null {
  if (value === 'info' || value === 'warn' || value === 'block') {
    return value
  }
  return null
}

function extractReviewReportLike(value: unknown): AnyRecord | null {
  const root = asRecord(value)
  if (!root) return null

  const looksLikeReport = typeof root.overall_status === 'string' && Array.isArray(root.issues)
  if (looksLikeReport) {
    return root
  }

  const data = asRecord(root.data)
  if (data) {
    const fromData = extractReviewReportLike(data)
    if (fromData) return fromData
  }

  for (const key of ['review_report', 'report', 'review', 'latest', 'reviewResult', 'review_result']) {
    const nested = asRecord(root[key])
    if (!nested) continue
    const extracted = extractReviewReportLike(nested)
    if (extracted) return extracted
  }

  const result = asRecord(root.result)
  const preview = asRecord(result?.preview)
  if (preview) {
    const fromPreview = extractReviewReportLike(preview)
    if (fromPreview) return fromPreview
  }

  return null
}

export function buildReviewCheckPreview(value: unknown): Record<string, unknown> | null {
  const report = extractReviewReportLike(value)
  if (!report) {
    const direct = asRecord(value)
    if (!direct || typeof direct.overall_status !== 'string') {
      return null
    }

    const issueCounts = asRecord(direct.issue_counts)
    const issuesTop = Array.isArray(direct.issues_top) ? direct.issues_top : undefined
    const issues = Array.isArray(direct.issues) ? direct.issues : undefined

    return {
      review_id: typeof direct.review_id === 'string' ? direct.review_id : undefined,
      overall_status: direct.overall_status,
      recommended_action: typeof direct.recommended_action === 'string' ? direct.recommended_action : undefined,
      generated_at: typeof direct.generated_at === 'number' ? direct.generated_at : undefined,
      review_types: Array.isArray(direct.review_types)
        ? direct.review_types.filter((v): v is string => typeof v === 'string').slice(0, 12)
        : undefined,
      issue_counts: issueCounts ? issueCounts : undefined,
      issues_top: Array.isArray(issuesTop)
        ? issuesTop
        : Array.isArray(issues)
          ? issues.slice(0, 12)
          : undefined,
    }
  }

  const issuesRaw = Array.isArray(report.issues) ? report.issues : []
  const issues = issuesRaw
    .map((issue) => asRecord(issue))
    .filter((issue): issue is AnyRecord => Boolean(issue))

  const counts = issues.reduce<{ info: number; warn: number; block: number; total: number }>(
    (acc, issue) => {
      const severity = normalizeIssueSeverity(issue.severity)
      if (severity === 'block') acc.block += 1
      else if (severity === 'warn') acc.warn += 1
      else if (severity === 'info') acc.info += 1
      acc.total += 1
      return acc
    },
    { info: 0, warn: 0, block: 0, total: 0 },
  )

  const top = issues.slice(0, 12).map((issue) => ({
    issue_id: typeof issue.issue_id === 'string' ? issue.issue_id : undefined,
    review_type: typeof issue.review_type === 'string' ? issue.review_type : undefined,
    severity: normalizeIssueSeverity(issue.severity) ?? undefined,
    summary: typeof issue.summary === 'string' ? issue.summary : undefined,
    auto_fixable: typeof issue.auto_fixable === 'boolean' ? issue.auto_fixable : undefined,
    evidence_refs: Array.isArray(issue.evidence_refs)
      ? issue.evidence_refs.filter((v): v is string => typeof v === 'string').slice(0, 3)
      : undefined,
    suggested_fix: typeof issue.suggested_fix === 'string' ? issue.suggested_fix : undefined,
  }))

  return {
    review_id: typeof report.review_id === 'string' ? report.review_id : undefined,
    overall_status: typeof report.overall_status === 'string' ? report.overall_status : undefined,
    recommended_action: typeof report.recommended_action === 'string' ? report.recommended_action : undefined,
    generated_at: typeof report.generated_at === 'number' ? report.generated_at : undefined,
    review_types: Array.isArray(report.review_types)
      ? report.review_types.filter((v): v is string => typeof v === 'string').slice(0, 12)
      : undefined,
    issue_counts: counts,
    issues_top: top,
  }
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
    review_check: ['scope_ref', 'target_refs', 'review_types', 'severity_threshold'],
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
  if (toolName === 'review_check') {
    if (trace.status !== 'ok') {
      return joinSummary('review error', [trace.fault_domain, trace.error_code])
    }

    const preview = buildReviewCheckPreview(parsedOutput)
    const status = typeof preview?.overall_status === 'string'
      ? preview.overall_status
      : (typeof (asRecord(parsedOutput)?.overall_status) === 'string'
        ? String(asRecord(parsedOutput)?.overall_status)
        : undefined)
    const counts = asRecord(preview?.issue_counts)
    const block = typeof counts?.block === 'number' ? counts.block : undefined
    const warn = typeof counts?.warn === 'number' ? counts.warn : undefined
    const action = typeof preview?.recommended_action === 'string' ? preview.recommended_action : undefined

    return joinSummary(`review ${status || 'ok'}`, [
      typeof block === 'number' ? `block=${block}` : null,
      typeof warn === 'number' ? `warn=${warn}` : null,
      action ? `action=${action}` : null,
    ])
  }

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

  if (input.toolName === 'review_check') {
    const preview = buildReviewCheckPreview(parsedOutput)
    return {
      parsedOutput,
      outputPreview: preview ? redactValue(preview) : redactValue(parsedOutput),
      rawOutput: trimRawOutput(input.output),
      retryable: extractRetryable(parsedOutput),
      errorMessage: extractErrorMessage(parsedOutput, input.trace),
      resultSummary,
    }
  }

  return {
    parsedOutput,
    outputPreview: redactValue(parsedOutput),
    rawOutput: trimRawOutput(input.output),
    retryable: extractRetryable(parsedOutput),
    errorMessage: extractErrorMessage(parsedOutput, input.trace),
    resultSummary,
  }
}

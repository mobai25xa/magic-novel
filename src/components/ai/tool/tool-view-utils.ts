import type { AgentUiToolStep } from '@/lib/agent-chat/types'

function isRecord(input: unknown): input is Record<string, unknown> {
  return Boolean(input) && typeof input === 'object' && !Array.isArray(input)
}

export function resolveStepPath(step: AgentUiToolStep): string | null {
  const preview = step.inputPreview
  if (!isRecord(preview)) return null

  const path = preview.path
  if (typeof path === 'string' && path.trim()) return path

  const chapterPath = preview.chapter_path
  if (typeof chapterPath === 'string' && chapterPath.trim()) return chapterPath

  const newChapterPath = preview.new_chapter_path
  if (typeof newChapterPath === 'string' && newChapterPath.trim()) return newChapterPath

  return null
}

function resolvePreviewOpenRef(preview: unknown): string | null {
  if (!isRecord(preview)) return null

  const candidates = [
    preview.open_ref,
    preview.target_ref,
    preview.path,
    preview.chapter_path,
    preview.new_chapter_path,
    preview.ref,
  ]

  for (const candidate of candidates) {
    if (typeof candidate === 'string' && candidate.trim()) {
      return candidate.trim()
    }
  }

  return null
}

export function resolveStepOpenRef(step: AgentUiToolStep): string | null {
  const parsed = parseToolOutput(step.rawOutput)

  return resolvePreviewOpenRef(step.outputPreview)
    || resolvePreviewOpenRef(parsed)
    || resolvePreviewOpenRef(step.inputPreview)
    || resolveStepPath(step)
}

export function resolveStepArgsSummary(step: AgentUiToolStep): string {
  if (step.argsSummary) return step.argsSummary

  const path = resolveStepPath(step)
  if (path) return path

  const preview = step.inputPreview
  if (isRecord(preview)) {
    const query = preview.query ?? preview.pattern ?? preview.keyword
    if (typeof query === 'string') return `"${query}"`

    const title = preview.title ?? preview.name
    if (typeof title === 'string') return title
  }

  return ''
}

export function buildStepErrorText(step: AgentUiToolStep): string {
  const parts = [
    step.toolName ? `tool=${step.toolName}` : null,
    step.faultDomain ? `fault=${step.faultDomain}` : null,
    step.errorCode ? `code=${step.errorCode}` : null,
    step.errorMessage ? `message=${step.errorMessage}` : null,
  ].filter(Boolean)

  return parts.length ? parts.join('\n') : 'Unknown tool error'
}

export async function copyStepPreview(value: unknown): Promise<boolean> {
  try {
    const text = typeof value === 'string' ? value : formatPreviewText(value)
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    return false
  }
}

export function formatPreviewText(value: unknown): string {
  if (typeof value === 'string') return value
  if (value === null || value === undefined) return ''

  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

export function resolveStepDurationMs(step: AgentUiToolStep, now: number): number {
  if (typeof step.durationMs === 'number') return step.durationMs
  if (typeof step.startedAt !== 'number') return 0

  const end = typeof step.finishedAt === 'number' ? step.finishedAt : now
  return Math.max(0, end - step.startedAt)
}

export function formatDurationMs(durationMs: number): string {
  if (!Number.isFinite(durationMs) || durationMs <= 0) return '--'
  if (durationMs < 1000) return `${Math.round(durationMs)}ms`
  return `${(durationMs / 1000).toFixed(1)}s`
}

export function parseToolOutput(rawOutput?: string): Record<string, unknown> | null {
  if (!rawOutput) return null

  try {
    const parsed = JSON.parse(rawOutput)
    if (typeof parsed === 'object' && parsed !== null) {
      return parsed as Record<string, unknown>
    }
  } catch {
    // not JSON
  }

  return null
}

export function isDryRun(step: AgentUiToolStep): boolean {
  const preview = step.inputPreview
  if (!isRecord(preview)) return false
  return preview.dry_run === true
}

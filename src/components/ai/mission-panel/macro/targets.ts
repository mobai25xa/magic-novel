import { asRecord, asString } from '../utils'

export type MacroChapterTargetInput = {
  chapter_ref: string
  write_path: string
  display_title?: string
}

export function normalizeMacroWritePathForUi(raw: string) {
  let path = raw.trim().replace(/\\/g, '/')
  while (path.startsWith('./')) {
    path = path.slice(2)
  }
  while (path.includes('//')) {
    path = path.replace(/\/\/+/, '/')
  }
  if (path.startsWith('manuscripts/')) {
    path = path.slice('manuscripts/'.length)
  }
  return path.trim()
}

function normalizeTarget(raw: unknown): MacroChapterTargetInput | null {
  if (typeof raw === 'string') {
    const write_path = normalizeMacroWritePathForUi(raw)
    if (!write_path) {
      return null
    }
    const chapter_ref = write_path.replace(/\.json$/i, '') || write_path
    return { chapter_ref, write_path }
  }

  const record = asRecord(raw)
  if (!record) {
    return null
  }

  const chapterRef = asString(record.chapter_ref ?? record.chapterRef)
  const writePath = asString(record.write_path ?? record.writePath)
  const displayTitle = asString(record.display_title ?? record.displayTitle)

  const chapter_ref = chapterRef?.trim() ?? ''
  const write_path = normalizeMacroWritePathForUi(writePath ?? '')

  if (!chapter_ref || !write_path) {
    return null
  }

  return {
    chapter_ref,
    write_path,
    display_title: displayTitle,
  }
}

function validateTargets(targets: MacroChapterTargetInput[]): string | undefined {
  const seenWrite = new Set<string>()
  const seenRef = new Set<string>()

  for (const target of targets) {
    if (!target.write_path.toLowerCase().endsWith('.json')) {
      return `write_path must end with .json: ${target.write_path}`
    }
    if (seenWrite.has(target.write_path)) {
      return `duplicate write_path: ${target.write_path}`
    }
    if (seenRef.has(target.chapter_ref)) {
      return `duplicate chapter_ref: ${target.chapter_ref}`
    }

    seenWrite.add(target.write_path)
    seenRef.add(target.chapter_ref)
  }

  return undefined
}

function parseTargetsFromArray(rows: unknown[]): { targets: MacroChapterTargetInput[]; error?: string } {
  const targets = rows
    .map((row) => normalizeTarget(row))
    .filter((target): target is MacroChapterTargetInput => Boolean(target))

  if (targets.length === 0) {
    return { targets: [], error: 'chapter_targets must contain at least 1 valid target' }
  }

  const error = validateTargets(targets)
  return error ? { targets: [], error } : { targets }
}

export function parseMacroChapterTargetsText(text: string): { targets: MacroChapterTargetInput[]; error?: string } {
  const trimmed = text.trim()
  if (!trimmed) {
    return { targets: [], error: 'chapter_targets is required' }
  }

  try {
    const parsed = JSON.parse(trimmed) as unknown
    if (Array.isArray(parsed)) {
      return parseTargetsFromArray(parsed)
    }
  } catch {
    // fall back to line-based parsing
  }

  const lines = trimmed
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)

  // Accept a simple list of write_paths.
  return parseTargetsFromArray(lines)
}


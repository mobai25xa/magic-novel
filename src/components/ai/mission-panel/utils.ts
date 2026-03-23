export function normalizeFsPath(path: string) {
  return path.replace(/\\/g, '/').replace(/\/+$/, '')
}

export function isMissingFileError(error: unknown) {
  const text = String((error as { message?: unknown } | null)?.message ?? error ?? '')
  const lower = text.toLowerCase()
  return lower.includes('not found')
    || lower.includes('no such file')
    || lower.includes('os error 2')
}

export function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }

  return value as Record<string, unknown>
}

export function asString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined
  }

  const trimmed = value.trim()
  return trimmed || undefined
}

export function asNumber(value: unknown): number | undefined {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }

  return undefined
}

export function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return []
  }

  return value.filter((item): item is string => typeof item === 'string')
}


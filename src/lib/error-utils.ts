type RecordLike = Record<string, unknown>

function asRecord(input: unknown): RecordLike {
  if (input && typeof input === 'object') return input as RecordLike
  return {}
}

function asNonEmptyString(input: unknown): string | undefined {
  if (typeof input !== 'string') return undefined
  const text = input.trim()
  return text ? text : undefined
}

function extractCodeFromRecord(record: RecordLike): string | undefined {
  const details = asRecord(record.details)
  return asNonEmptyString(record.code)
    || asNonEmptyString(details.code)
    || asNonEmptyString((record.error as RecordLike | undefined)?.code)
}

function extractMessageFromRecord(record: RecordLike): string | undefined {
  const direct = asNonEmptyString(record.message)
  if (direct) return direct

  const errorObj = asRecord(record.error)
  const nestedMessage = asNonEmptyString(errorObj.message)
  if (nestedMessage) return nestedMessage

  const details = asRecord(record.details)
  const detailMessage = asNonEmptyString(details.message)
  if (detailMessage) return detailMessage

  return undefined
}

export function formatUnknownError(error: unknown, fallback = 'Unknown error'): string {
  if (error instanceof Error) {
    const message = error.message?.trim()
    if (message) return message
    return fallback
  }

  const record = asRecord(error)
  const message = extractMessageFromRecord(record)
  const code = extractCodeFromRecord(record)

  if (message && code) {
    return `${message} (${code})`
  }
  if (message) {
    return message
  }
  if (code) {
    return code
  }

  if (typeof error === 'string') {
    const text = error.trim()
    return text || fallback
  }

  if (error == null) {
    return fallback
  }

  try {
    const json = JSON.stringify(error)
    return json && json !== '{}' ? json : fallback
  } catch {
    return fallback
  }
}

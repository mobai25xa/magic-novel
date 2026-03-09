import { invoke } from '@tauri-apps/api/core'

type RecordLike = Record<string, unknown>

function asRecord(input: unknown): RecordLike {
  if (input && typeof input === 'object' && !Array.isArray(input)) {
    return input as RecordLike
  }

  return {}
}

function asMaybeNumber(input: unknown): number | undefined {
  if (typeof input === 'number' && Number.isFinite(input)) {
    return input
  }

  if (typeof input === 'string') {
    const value = Number(input)
    if (Number.isFinite(value)) {
      return value
    }
  }

  return undefined
}

function asMaybeString(input: unknown): string | undefined {
  if (typeof input !== 'string') {
    return undefined
  }

  const text = input.trim()
  return text || undefined
}

function truncateText(input: string, max = 4000): string {
  if (input.length <= max) {
    return input
  }

  return `${input.slice(0, max)}…<truncated>`
}

function normalizeBody(input: unknown): unknown {
  if (typeof input === 'string') {
    const text = input.trim()
    if (!text) {
      return ''
    }

    try {
      return JSON.parse(text)
    } catch {
      return truncateText(text)
    }
  }

  if (input == null) {
    return undefined
  }

  return input
}

function logInvokeError(command: string, payload: Record<string, unknown> | undefined, error: unknown) {
  const errorRecord = asRecord(error)
  const details = asRecord(errorRecord.details)
  const summary = asMaybeString(errorRecord.message)
    || asMaybeString(errorRecord.error)
    || (error instanceof Error ? error.message : undefined)
    || 'invoke failed'

  console.error('[tauri] invoke failed', {
    command,
    summary,
    code: asMaybeString(errorRecord.code) || asMaybeString(details.code),
    status: asMaybeNumber(details.status) || asMaybeNumber(errorRecord.status),
    requestId: asMaybeString(details.request_id) || asMaybeString(details.requestId) || asMaybeString(errorRecord.request_id),
    recoverable: typeof errorRecord.recoverable === 'boolean' ? errorRecord.recoverable : undefined,
    body: normalizeBody(details.body),
    details,
    payload,
    rawError: error,
  })
}

export async function invokeTauri<T>(command: string, payload?: Record<string, unknown>): Promise<T> {
  try {
    if (payload) {
      return await invoke<T>(command, payload)
    }

    return await invoke<T>(command)
  } catch (error) {
    logInvokeError(command, payload, error)
    throw error
  }
}

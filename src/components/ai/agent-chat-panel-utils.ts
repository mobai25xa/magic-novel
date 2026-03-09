import { formatUnknownError } from '@/lib/error-utils'

export type AgentPanelError = {
  summary: string
  code?: string
  status?: number
  body?: string
  faultDomain?: string
  details?: Record<string, unknown>
}

function asRecord(input: unknown): Record<string, unknown> | undefined {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return undefined
  }
  return input as Record<string, unknown>
}

function asNonEmptyString(input: unknown): string | undefined {
  if (typeof input !== 'string') {
    return undefined
  }
  const text = input.trim()
  return text || undefined
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

function normalizeBody(input: unknown): string | undefined {
  if (typeof input === 'string') {
    const text = input.trim()
    return text || undefined
  }

  if (input == null) {
    return undefined
  }

  try {
    const json = JSON.stringify(input, null, 2)
    return json || undefined
  } catch {
    return String(input)
  }
}

export function parseAgentError(error: unknown): AgentPanelError {
  const summary = formatUnknownError(error, 'UNKNOWN_ERROR')

  if (!error || typeof error !== 'object') {
    return { summary }
  }

  const candidate = error as Record<string, unknown>
  const details = asRecord(candidate.details)

  const code = asNonEmptyString(candidate.code)
    || asNonEmptyString(details?.code)

  const status = asMaybeNumber(details?.status)
    || asMaybeNumber(candidate.status)

  const body = normalizeBody(details?.body)

  const faultDomain = asNonEmptyString(candidate.fault_domain)
    || asNonEmptyString(details?.fault_domain)

  return {
    summary,
    code,
    status,
    body,
    faultDomain,
    details,
  }
}

export function parseAgentErrorCode(error: unknown): string {
  return parseAgentError(error).summary
}

export function toAgentPanelError(input: string | AgentPanelError | null | undefined): AgentPanelError | null {
  if (!input) {
    return null
  }

  if (typeof input === 'object' && typeof input.summary === 'string') {
    return input
  }

  return parseAgentError(input)
}

export function hasExpandableAgentErrorDetails(error: AgentPanelError): boolean {
  return Boolean(
    error.code
    || typeof error.status === 'number'
    || error.body
    || error.faultDomain
    || (error.details && Object.keys(error.details).length > 0),
  )
}

export function formatAgentErrorDetails(error: AgentPanelError): string {
  const payload = {
    code: error.code,
    status: error.status,
    fault_domain: error.faultDomain,
    body: error.body,
    details: error.details,
  }

  try {
    return JSON.stringify(payload, null, 2)
  } catch {
    return String(payload)
  }
}

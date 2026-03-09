import type { AgentUiTurnError, AgentUiToolStep } from '@/lib/agent-chat/types'

export type ErrorCategory =
  | 'auth'
  | 'rate_limit'
  | 'server'
  | 'network'
  | 'context_limit'
  | 'client'

export interface ErrorDescriptor {
  category: ErrorCategory
  code: string
  retryable: boolean
  provider?: string
  diagnostic?: string
  httpStatus?: number
  retryAfterMs?: number
}

export const TURN_ERROR_CODE_CATEGORY_MAP: Record<string, ErrorCategory> = {
  E_AUTH: 'auth',
  E_RATE_LIMIT: 'rate_limit',
  E_SERVER_ERROR: 'server',
  E_NETWORK: 'network',
  E_CONTEXT_LIMIT: 'context_limit',
  E_EMPTY_BODY: 'server',
  E_EMPTY_RESPONSE: 'server',
  E_PARSE_ERROR: 'client',
  E_LLM_UNKNOWN: 'server',
}

const RETRYABLE_CODES = new Set([
  'E_RATE_LIMIT', 'E_SERVER_ERROR', 'E_NETWORK', 'E_EMPTY_BODY', 'E_EMPTY_RESPONSE',
])

const VALID_CATEGORIES: ReadonlySet<string> = new Set([
  'auth', 'rate_limit', 'server', 'network', 'context_limit', 'client',
])

function isValidCategory(value: string): value is ErrorCategory {
  return VALID_CATEGORIES.has(value)
}

export function classifyTurnError(turnError: AgentUiTurnError): ErrorDescriptor {
  const code = turnError.code || 'E_LLM_UNKNOWN'
  const hint = turnError.detail?.category_hint
  const category: ErrorCategory =
    (hint && isValidCategory(hint) ? hint : undefined)
    ?? TURN_ERROR_CODE_CATEGORY_MAP[code]
    ?? 'server'

  return {
    category,
    code,
    retryable: turnError.detail?.retryable ?? RETRYABLE_CODES.has(code),
    provider: turnError.detail?.provider,
    diagnostic: turnError.detail?.diagnostic,
    httpStatus: turnError.detail?.http_status,
    retryAfterMs: turnError.detail?.retry_after_ms,
  }
}

const FAULT_DOMAIN_CATEGORY_MAP: Record<string, ErrorCategory> = {
  validation: 'client',
  policy: 'client',
  tool: 'client',
  io: 'network',
  network: 'network',
  auth: 'auth',
  jvm: 'server',
  vc: 'server',
  external: 'server',
}

export function classifyToolError(step: AgentUiToolStep): ErrorDescriptor {
  const code = step.errorCode || 'E_TOOL_UNKNOWN'
  const category = (step.faultDomain && FAULT_DOMAIN_CATEGORY_MAP[step.faultDomain])
    || 'client'

  return {
    category,
    code,
    retryable: step.retryable ?? false,
    diagnostic: step.errorMessage,
  }
}

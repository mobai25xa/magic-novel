import { createMessageId } from '@/agent/utils'

export function normalizeUserInput(userText: string) {
  return userText.trim()
}

export function createClientRequestId() {
  return `req_${createMessageId()}`
}

export function buildUserUiMessage(input: string, turn?: number) {
  return {
    id: createMessageId(),
    role: 'user' as const,
    content: input,
    ts: Date.now(),
    turn,
  }
}

export function parseConcurrencyError(error: unknown) {
  if (!error || typeof error !== 'object') {
    return 'E_AGENT_CONCURRENCY_LIMIT'
  }

  const candidate = error as { message?: string }
  return candidate.message || 'E_AGENT_CONCURRENCY_LIMIT'
}

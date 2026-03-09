export function createMessageId() {
  return `${Date.now()}_${Math.random().toString(36).slice(2, 8)}`
}

export function safeParseJson(input: string): Record<string, unknown> {
  try {
    const parsed = JSON.parse(input) as Record<string, unknown>
    if (parsed && typeof parsed === 'object') return parsed
    return {}
  } catch {
    return {}
  }
}

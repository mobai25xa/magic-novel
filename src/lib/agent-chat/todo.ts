import type { AgentTodoItem, AgentTodoState, AgentTodoStatus } from '@/agent/types'
import { extractTodoStateFromTrace } from './tool-trace'

export const MAX_TODO_ITEMS = 50
export const MAX_TODO_TEXT_LENGTH = 500

export type ParsedTodoWriteSuccess = {
  ok: true
  state: AgentTodoState
  normalizedText: string
}

export type ParsedTodoWriteError = {
  ok: false
  code: 'E_TOOL_SCHEMA_INVALID'
  message: string
}

export type ParsedTodoResult = ParsedTodoWriteSuccess | ParsedTodoWriteError

type TodoItemsParseResult = {
  items: AgentTodoItem[] | null
  error?: string
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }

  return value as Record<string, unknown>
}

function toMaybeString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined
  }

  const trimmed = value.trim()
  return trimmed || undefined
}

function normalizeStatus(raw: string): AgentTodoStatus | null {
  const value = raw.trim().toLowerCase()
  if (value === 'pending' || value === 'in_progress' || value === 'completed') {
    return value
  }
  return null
}

function normalizeInProgressItems(items: AgentTodoItem[]): AgentTodoItem[] {
  let inProgressSeen = false

  return items.map((item) => {
    if (item.status !== 'in_progress') {
      return item
    }

    if (!inProgressSeen) {
      inProgressSeen = true
      return item
    }

    return {
      ...item,
      status: 'pending',
    }
  })
}

function sanitizeTodoText(input: string): string {
  const text = input.trim()
  if (!text) {
    return ''
  }

  let value = text

  value = value.replace(/(?:[A-Za-z]:)?[\\/][^\s]{2,}/g, '[path]')
  value = value.replace(/\b(?:sk|pk|rk|api|token|secret)[-_]?[A-Za-z0-9]{8,}\b/gi, '[redacted]')
  value = value.replace(/\b[a-f0-9]{24,}\b/gi, '[redacted]')

  if (value.length > MAX_TODO_TEXT_LENGTH) {
    return value.slice(0, MAX_TODO_TEXT_LENGTH)
  }

  return value
}

function parseTodoItemsFromUnknown(input: {
  todos: unknown
  allowEmpty: boolean
}): TodoItemsParseResult {
  if (!Array.isArray(input.todos)) {
    return {
      items: null,
      error: 'todos must be an array',
    }
  }

  if (!input.allowEmpty && input.todos.length === 0) {
    return {
      items: null,
      error: 'No todo item found',
    }
  }

  if (input.todos.length > MAX_TODO_ITEMS) {
    return {
      items: null,
      error: `too many todo items (max ${MAX_TODO_ITEMS})`,
    }
  }

  const items: AgentTodoItem[] = []
  for (const rowValue of input.todos) {
    const row = asRecord(rowValue)
    if (!row) {
      return {
        items: null,
        error: 'Invalid todo item format. Use: { status, text }',
      }
    }

    const status = normalizeStatus(String(row.status ?? ''))
    if (!status) {
      return {
        items: null,
        error: 'Invalid status. Allowed: pending, in_progress, completed',
      }
    }

    const text = typeof row.text === 'string' ? sanitizeTodoText(row.text) : ''
    if (!text) {
      return {
        items: null,
        error: 'todo text must not be empty',
      }
    }

    items.push({ status, text })
  }

  return {
    items: normalizeInProgressItems(items),
  }
}

export function parseTodoWriteInput(input: {
  todos: unknown
  callId: string
  now?: number
}): ParsedTodoResult {
  return parseTodoWriteInputV2(input)
}

export function parseTodoWriteInputV2(input: {
  todos: unknown
  callId: string
  now?: number
}): ParsedTodoResult {
  const parsed = parseTodoItemsFromUnknown({
    todos: input.todos,
    allowEmpty: false,
  })

  if (!parsed.items) {
    return {
      ok: false,
      code: 'E_TOOL_SCHEMA_INVALID',
      message: parsed.error || 'Invalid todos input',
    }
  }

  return {
    ok: true,
    state: {
      items: parsed.items,
      lastUpdatedAt: input.now ?? Date.now(),
      sourceCallId: input.callId,
    },
    normalizedText: stringifyTodoItems(parsed.items),
  }
}

export function stringifyTodoItems(items: AgentTodoItem[]) {
  return items
    .map((item, index) => `${index + 1}. [${item.status}] ${item.text}`)
    .join('\n')
}

export function normalizeTodoStateFromTrace(
  trace?: Record<string, unknown>,
  fallbackCallId?: string,
): AgentTodoState | null {
  return extractTodoStateFromTrace({
    trace,
    fallbackCallId,
  })
}

export function normalizeTodoStateFromToolResultPayload(payload?: Record<string, unknown>): AgentTodoState | null {
  if (!payload || typeof payload !== 'object') {
    return null
  }

  const toolName = toMaybeString(payload.tool_name)
  if (toolName && toolName.toLowerCase() !== 'todowrite') {
    return null
  }

  const callId = toMaybeString(payload.call_id) || toMaybeString(payload.llm_call_id)
  return normalizeTodoStateFromTrace(asRecord(payload.trace) || undefined, callId)
}

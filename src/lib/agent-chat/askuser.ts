import type {
  AgentAskUserQuestion,
  AgentPendingAskUserRequest,
} from '@/agent/types'

export const ASKUSER_MIN_QUESTIONS = 1
export const ASKUSER_MAX_QUESTIONS = 4
export const ASKUSER_MIN_OPTIONS = 2
export const ASKUSER_MAX_OPTIONS = 4

export function normalizeAskUserToolName(toolName: unknown) {
  return String(toolName ?? '').trim().toLowerCase()
}

export function isAskUserToolName(toolName: unknown) {
  const normalized = normalizeAskUserToolName(toolName)
  return normalized === 'askuser'
}

const QUESTION_HEADER_RE = /^\s*(\d+)\.\s*\[question\]\s*(.+?)\s*$/i
const TOPIC_LINE_RE = /^\s*\[topic\]\s*(.+?)\s*$/i
const OPTION_LINE_RE = /^\s*\[option\]\s*(.+?)\s*$/i

type ParseFailureCode =
  | 'E_TOOL_SCHEMA_INVALID'

export type AskUserParseResult =
  | {
    ok: true
    questionnaire: string
    questions: AgentAskUserQuestion[]
  }
  | {
    ok: false
    code: ParseFailureCode
    message: string
  }

export type AskUserAnswersResult = {
  answers: Array<{ topic: string; value: string }>
}

function sanitizeLine(line: string) {
  return line.replace(/\r/g, '').trim()
}

function parseQuestionHeader(line: string): { index: number; question: string } | null {
  const match = line.match(QUESTION_HEADER_RE)
  if (!match) {
    return null
  }

  const index = Number(match[1])
  const question = sanitizeLine(match[2] || '')
  if (!Number.isFinite(index) || index <= 0 || !question) {
    return null
  }

  return {
    index,
    question,
  }
}

function parseTopicLine(line: string): string | null {
  const match = line.match(TOPIC_LINE_RE)
  if (!match) {
    return null
  }

  const topic = sanitizeLine(match[1] || '')
  return topic || null
}

function parseOptionLine(line: string): string | null {
  const match = line.match(OPTION_LINE_RE)
  if (!match) {
    return null
  }

  const option = sanitizeLine(match[1] || '')
  return option || null
}

export function mapStructuredAskUserQuestions(raw: unknown): AgentAskUserQuestion[] | null {
  if (!Array.isArray(raw) || raw.length === 0 || raw.length > ASKUSER_MAX_QUESTIONS) {
    return null
  }

  const result: AgentAskUserQuestion[] = []
  for (let index = 0; index < raw.length; index += 1) {
    const item = raw[index]
    if (!item || typeof item !== 'object' || Array.isArray(item)) {
      return null
    }

    const candidate = item as Record<string, unknown>
    const question = typeof candidate.question === 'string' ? candidate.question.trim() : ''
    const topic = typeof candidate.topic === 'string' ? candidate.topic.trim() : ''
    const options = Array.isArray(candidate.options)
      ? candidate.options.filter((option): option is string => typeof option === 'string' && option.trim().length > 0)
      : []

    if (!question || !topic || options.length < ASKUSER_MIN_OPTIONS || options.length > ASKUSER_MAX_OPTIONS) {
      return null
    }

    result.push({
      index,
      question,
      topic,
      options,
    })
  }

  return result
}

function parseQuestionBlock(lines: string[], start: number) {
  const header = parseQuestionHeader(lines[start])
  if (!header) {
    return {
      ok: false as const,
      message: 'Invalid question header. Use: 1. [question] ...',
    }
  }

  let cursor = start + 1
  while (cursor < lines.length && !sanitizeLine(lines[cursor])) {
    cursor += 1
  }

  if (cursor >= lines.length) {
    return {
      ok: false as const,
      message: `Question ${header.index} is missing [topic]`,
    }
  }

  const topic = parseTopicLine(lines[cursor])
  if (!topic) {
    return {
      ok: false as const,
      message: `Question ${header.index} requires [topic] line`,
    }
  }

  cursor += 1

  const options: string[] = []
  while (cursor < lines.length) {
    const raw = lines[cursor]
    const text = sanitizeLine(raw)

    if (!text) {
      cursor += 1
      if (cursor < lines.length && parseQuestionHeader(lines[cursor])) {
        break
      }
      continue
    }

    if (parseQuestionHeader(raw)) {
      break
    }

    const option = parseOptionLine(raw)
    if (!option) {
      return {
        ok: false as const,
        message: `Question ${header.index} has invalid option line: ${text}`,
      }
    }

    options.push(option)
    cursor += 1

    if (options.length > ASKUSER_MAX_OPTIONS) {
      return {
        ok: false as const,
        message: `Question ${header.index} has too many options (max ${ASKUSER_MAX_OPTIONS})`,
      }
    }
  }

  if (options.length < ASKUSER_MIN_OPTIONS) {
    return {
      ok: false as const,
      message: `Question ${header.index} requires at least ${ASKUSER_MIN_OPTIONS} options`,
    }
  }

  return {
    ok: true as const,
    next: cursor,
    question: {
      index: header.index,
      question: header.question,
      topic,
      options,
    } as AgentAskUserQuestion,
  }
}

export function parseAskUserQuestionnaire(questionnaire: unknown): AskUserParseResult {
  if (typeof questionnaire !== 'string') {
    return {
      ok: false,
      code: 'E_TOOL_SCHEMA_INVALID',
      message: 'questionnaire must be a string',
    }
  }

  const normalized = questionnaire
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .trim()

  if (!normalized) {
    return {
      ok: false,
      code: 'E_TOOL_SCHEMA_INVALID',
      message: 'questionnaire must not be empty',
    }
  }

  const lines = normalized.split('\n')
  const questions: AgentAskUserQuestion[] = []

  let cursor = 0
  while (cursor < lines.length) {
    while (cursor < lines.length && !sanitizeLine(lines[cursor])) {
      cursor += 1
    }

    if (cursor >= lines.length) {
      break
    }

    const block = parseQuestionBlock(lines, cursor)
    if (!block.ok) {
      return {
        ok: false,
        code: 'E_TOOL_SCHEMA_INVALID',
        message: block.message,
      }
    }

    questions.push(block.question)
    cursor = block.next

    if (questions.length > ASKUSER_MAX_QUESTIONS) {
      return {
        ok: false,
        code: 'E_TOOL_SCHEMA_INVALID',
        message: `too many questions (max ${ASKUSER_MAX_QUESTIONS})`,
      }
    }
  }

  if (questions.length < ASKUSER_MIN_QUESTIONS) {
    return {
      ok: false,
      code: 'E_TOOL_SCHEMA_INVALID',
      message: `at least ${ASKUSER_MIN_QUESTIONS} question is required`,
    }
  }

  for (let index = 0; index < questions.length; index += 1) {
    const expected = index + 1
    if (questions[index].index !== expected) {
      return {
        ok: false,
        code: 'E_TOOL_SCHEMA_INVALID',
        message: `question index must be continuous and start at 1 (found ${questions[index].index}, expected ${expected})`,
      }
    }
  }

  return {
    ok: true,
    questionnaire: normalized,
    questions,
  }
}

export function validateAskUserAnswers(input: {
  request: AgentPendingAskUserRequest
  values: unknown
}): AskUserAnswersResult | null {
  if (!Array.isArray(input.values)) {
    return null
  }

  if (input.values.length !== input.request.questions.length) {
    return null
  }

  const answers: Array<{ topic: string; value: string }> = []

  for (let index = 0; index < input.request.questions.length; index += 1) {
    const expectedQuestion = input.request.questions[index]
    const row = input.values[index]
    if (!row || typeof row !== 'object' || Array.isArray(row)) {
      return null
    }

    const candidate = row as Record<string, unknown>
    const topic = typeof candidate.topic === 'string' ? candidate.topic.trim() : ''
    const value = typeof candidate.value === 'string' ? candidate.value.trim() : ''

    if (!topic || !value || topic !== expectedQuestion.topic) {
      return null
    }

    answers.push({ topic, value })
  }

  return {
    answers,
  }
}

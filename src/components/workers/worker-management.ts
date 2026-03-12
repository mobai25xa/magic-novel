import type { WorkerDefinition } from '@/features/global-config'

export type WorkerFormValue = {
  name: string
  display_name: string
  system_prompt: string
  tool_whitelist: string[]
  match_keywords: string
  max_rounds: string
  max_tool_calls: string
  model: string
}

export const AVAILABLE_WORKER_TOOLS = [
  'read',
  'edit',
  'create',
  'ls',
  'grep',
  'review_check',
  'outline',
  'character_sheet',
  'search_knowledge',
] as const

export function createEmptyWorkerForm(): WorkerFormValue {
  return {
    name: '',
    display_name: '',
    system_prompt: '',
    tool_whitelist: ['read', 'grep'],
    match_keywords: '',
    max_rounds: '',
    max_tool_calls: '',
    model: '',
  }
}

export function workerToFormValue(worker: WorkerDefinition): WorkerFormValue {
  return {
    name: worker.name,
    display_name: worker.display_name,
    system_prompt: worker.system_prompt,
    tool_whitelist: [...worker.tool_whitelist],
    match_keywords: worker.match_keywords.join(', '),
    max_rounds: worker.max_rounds ? String(worker.max_rounds) : '',
    max_tool_calls: worker.max_tool_calls ? String(worker.max_tool_calls) : '',
    model: worker.model ?? '',
  }
}

export function parseWorkerFormValue(form: WorkerFormValue): WorkerDefinition {
  const maxRounds = parsePositiveInt(form.max_rounds)
  const maxToolCalls = parsePositiveInt(form.max_tool_calls)
  const model = form.model.trim()

  return {
    name: form.name.trim(),
    display_name: form.display_name.trim(),
    system_prompt: form.system_prompt.trim(),
    tool_whitelist: form.tool_whitelist.map((tool) => tool.trim()).filter(Boolean),
    match_keywords: form.match_keywords
      .split(',')
      .map((value) => value.trim())
      .filter(Boolean),
    max_rounds: maxRounds,
    max_tool_calls: maxToolCalls,
    model: model ? model : undefined,
  }
}

export function validateWorkerForm(
  form: WorkerFormValue,
  labels: {
    requiredName: string
    requiredDisplayName: string
    requiredPrompt: string
    requiredTools: string
  },
): string | null {
  if (!form.name.trim()) return labels.requiredName
  if (!form.display_name.trim()) return labels.requiredDisplayName
  if (!form.system_prompt.trim()) return labels.requiredPrompt
  if (form.tool_whitelist.length === 0) return labels.requiredTools
  return null
}

export function toggleTool(form: WorkerFormValue, tool: string): WorkerFormValue {
  const next = new Set(form.tool_whitelist)
  if (next.has(tool)) {
    next.delete(tool)
  } else {
    next.add(tool)
  }
  return {
    ...form,
    tool_whitelist: Array.from(next),
  }
}

export function safeParseWorkerJson(content: string): WorkerDefinition | null {
  try {
    const value = JSON.parse(content) as Partial<WorkerDefinition>
    if (!value || typeof value !== 'object') return null
    if (!value.name || !value.display_name || !value.system_prompt || !Array.isArray(value.tool_whitelist)) {
      return null
    }

    return {
      name: String(value.name),
      display_name: String(value.display_name),
      system_prompt: String(value.system_prompt),
      tool_whitelist: value.tool_whitelist.map((tool) => String(tool)),
      match_keywords: Array.isArray(value.match_keywords)
        ? value.match_keywords.map((keyword) => String(keyword))
        : [],
      max_rounds:
        typeof value.max_rounds === 'number' && value.max_rounds > 0
          ? Math.floor(value.max_rounds)
          : undefined,
      max_tool_calls:
        typeof value.max_tool_calls === 'number' && value.max_tool_calls > 0
          ? Math.floor(value.max_tool_calls)
          : undefined,
      model: typeof value.model === 'string' && value.model.trim() ? value.model.trim() : undefined,
    }
  } catch {
    return null
  }
}

function parsePositiveInt(value: string): number | undefined {
  const trimmed = value.trim()
  if (!trimmed) return undefined
  const parsed = Number(trimmed)
  if (!Number.isFinite(parsed) || parsed <= 0) return undefined
  return Math.floor(parsed)
}

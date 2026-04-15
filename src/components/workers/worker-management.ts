import {
  type ApprovalMode,
  type CapabilityMode,
  type ClarificationMode,
  WORKER_CAPABILITY_PRESETS,
  type CapabilityPreset,
  type WorkerDefinition,
} from '@/features/global-config'

export type WorkerFormValue = {
  name: string
  display_name: string
  system_prompt: string
  mode: CapabilityMode
  approval_mode: ApprovalMode
  clarification_mode: ClarificationMode
  capability_preset: CapabilityPreset
  allow_delegate: boolean
  allow_skill_activation: boolean
  hidden_tools: string
  forced_tools: string
  max_rounds: string
  max_tool_calls: string
  model: string
}

export const AVAILABLE_WORKER_PRESETS = WORKER_CAPABILITY_PRESETS

export function createEmptyWorkerForm(): WorkerFormValue {
  return {
    name: '',
    display_name: '',
    system_prompt: '',
    mode: 'writing',
    approval_mode: 'auto',
    clarification_mode: 'headless_defer',
    capability_preset: 'headless_writer',
    allow_delegate: false,
    allow_skill_activation: false,
    hidden_tools: '',
    forced_tools: '',
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
    mode: worker.mode,
    approval_mode: worker.approval_mode,
    clarification_mode: worker.clarification_mode,
    capability_preset: worker.capability_preset,
    allow_delegate: worker.allow_delegate,
    allow_skill_activation: worker.allow_skill_activation,
    hidden_tools: worker.hidden_tools.join(', '),
    forced_tools: worker.forced_tools.join(', '),
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
    mode: form.mode,
    approval_mode: form.approval_mode,
    clarification_mode: form.clarification_mode,
    capability_preset: form.capability_preset,
    allow_delegate: form.allow_delegate,
    allow_skill_activation: form.allow_skill_activation,
    hidden_tools: splitCsv(form.hidden_tools),
    forced_tools: splitCsv(form.forced_tools),
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
  },
): string | null {
  if (!form.name.trim()) return labels.requiredName
  if (!form.display_name.trim()) return labels.requiredDisplayName
  if (!form.system_prompt.trim()) return labels.requiredPrompt
  return null
}

export function safeParseWorkerJson(content: string): WorkerDefinition | null {
  try {
    const value = JSON.parse(content) as Partial<WorkerDefinition>
    if (!value || typeof value !== 'object') return null
    if (!value.name || !value.display_name || !value.system_prompt) {
      return null
    }

    return {
      name: String(value.name),
      display_name: String(value.display_name),
      system_prompt: String(value.system_prompt),
      mode: value.mode === 'planning' ? 'planning' : 'writing',
      approval_mode: value.approval_mode === 'confirm_writes' ? 'confirm_writes' : 'auto',
      clarification_mode:
        value.clarification_mode === 'interactive' ? 'interactive' : 'headless_defer',
      capability_preset: isCapabilityPreset(value.capability_preset)
        ? value.capability_preset
        : 'headless_writer',
      allow_delegate: Boolean(value.allow_delegate),
      allow_skill_activation: Boolean(value.allow_skill_activation),
      hidden_tools: Array.isArray(value.hidden_tools)
        ? value.hidden_tools.map((tool) => String(tool))
        : [],
      forced_tools: Array.isArray(value.forced_tools)
        ? value.forced_tools.map((tool) => String(tool))
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

function splitCsv(value: string): string[] {
  return value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
}

function parsePositiveInt(value: string): number | undefined {
  const trimmed = value.trim()
  if (!trimmed) return undefined
  const parsed = Number(trimmed)
  if (!Number.isFinite(parsed) || parsed <= 0) return undefined
  return Math.floor(parsed)
}

function isCapabilityPreset(value: unknown): value is CapabilityPreset {
  return WORKER_CAPABILITY_PRESETS.includes(value as CapabilityPreset)
}

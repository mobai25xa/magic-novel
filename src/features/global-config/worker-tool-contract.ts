import type { WorkerDefinition, CapabilityPreset } from '@/platform/tauri/clients/global-config-client'

export const BUILTIN_WORKER_TOOL_NAMES = [
  'workspace_map',
  'context_read',
  'context_search',
  'knowledge_read',
  'knowledge_write',
  'draft_write',
  'structure_edit',
  'review_check',
  'skill',
  'todowrite',
] as const

export type BuiltinWorkerToolName = typeof BUILTIN_WORKER_TOOL_NAMES[number]

export const WORKER_CAPABILITY_PRESETS: CapabilityPreset[] = [
  'headless_writer',
  'read_only_reviewer',
  'summary_only',
  'main_planning',
  'main_interactive',
]

export function isBuiltinWorkerToolName(value: string): value is BuiltinWorkerToolName {
  return BUILTIN_WORKER_TOOL_NAMES.includes(value as BuiltinWorkerToolName)
}

export function toolsForCapabilityPreset(preset: CapabilityPreset): BuiltinWorkerToolName[] {
  switch (preset) {
    case 'main_interactive':
      return [
        'workspace_map',
        'context_read',
        'context_search',
        'knowledge_read',
        'knowledge_write',
        'draft_write',
        'structure_edit',
        'review_check',
        'todowrite',
      ]
    case 'main_planning':
      return ['workspace_map', 'context_read', 'context_search', 'knowledge_read', 'review_check', 'todowrite']
    case 'headless_writer':
      return [
        'workspace_map',
        'context_read',
        'context_search',
        'knowledge_read',
        'knowledge_write',
        'draft_write',
        'structure_edit',
        'review_check',
        'todowrite',
      ]
    case 'read_only_reviewer':
      return ['workspace_map', 'context_read', 'context_search', 'knowledge_read', 'review_check', 'todowrite']
    case 'summary_only':
      return ['workspace_map', 'context_read', 'context_search', 'knowledge_read', 'review_check']
  }
}

export function resolveWorkerVisibleTools(worker: WorkerDefinition): string[] {
  const hidden = new Set(worker.hidden_tools)
  const forced = new Set(worker.forced_tools)
  const visible: string[] = toolsForCapabilityPreset(worker.capability_preset).filter((tool) => !hidden.has(tool))

  if (worker.allow_skill_activation) {
    visible.push('skill')
  }

  forced.forEach((tool) => {
    if (!hidden.has(tool)) {
      visible.push(tool)
    }
  })

  return Array.from(new Set(visible))
}

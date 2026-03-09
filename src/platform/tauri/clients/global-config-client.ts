import { invokeTauri } from './core'

// ── Types (mirror Rust DTOs) ─────────────────────────────────────

export type SkillSource = 'builtin' | 'user'

export interface SkillDefinition {
  name: string
  display_name: string
  description: string
  system_prompt_snippet: string
  enabled: boolean
  source: SkillSource
}

export interface WorkerDefinition {
  name: string
  display_name: string
  system_prompt: string
  tool_whitelist: string[]
  match_keywords: string[]
  max_rounds?: number
  max_tool_calls?: number
  model?: string
}

// ── Tauri invoke wrappers ────────────────────────────────────────

export async function listSkillsClient(): Promise<SkillDefinition[]> {
  return invokeTauri<SkillDefinition[]>('list_skills')
}

export async function saveSkillClient(name: string, content: string): Promise<void> {
  return invokeTauri<void>('save_skill', { name, content })
}

export async function deleteSkillClient(name: string): Promise<void> {
  return invokeTauri<void>('delete_skill', { name })
}

export async function importSkillClient(inputPath: string, overrideName?: string): Promise<string> {
  return invokeTauri<string>('import_skill', {
    inputPath,
    overrideName: overrideName?.trim() ? overrideName.trim() : null,
  })
}

export async function exportSkillClient(name: string, outputPath: string): Promise<void> {
  return invokeTauri<void>('export_skill', { name, outputPath })
}

export async function listWorkersClient(): Promise<WorkerDefinition[]> {
  return invokeTauri<WorkerDefinition[]>('list_workers')
}

export async function saveWorkerClient(definition: WorkerDefinition): Promise<void> {
  return invokeTauri<void>('save_worker', { definition })
}

export async function deleteWorkerClient(name: string): Promise<void> {
  return invokeTauri<void>('delete_worker', { name })
}

export async function getGlobalRulesClient(): Promise<string | null> {
  return invokeTauri<string | null>('get_global_rules')
}

export async function saveGlobalRulesClient(content: string): Promise<void> {
  return invokeTauri<void>('save_global_rules', { content })
}

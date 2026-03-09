import {
  deleteSkillClient,
  deleteWorkerClient,
  exportSkillClient,
  getGlobalRulesClient,
  importSkillClient,
  listSkillsClient,
  listWorkersClient,
  saveGlobalRulesClient,
  saveSkillClient,
  saveWorkerClient,
  type SkillDefinition,
  type WorkerDefinition,
} from '@/platform/tauri/clients/global-config-client'

export type { SkillDefinition, WorkerDefinition }

export async function listSkillsFeature(): Promise<SkillDefinition[]> {
  return listSkillsClient()
}

export async function saveSkillFeature(name: string, content: string): Promise<void> {
  await saveSkillClient(name, content)
}

export async function deleteSkillFeature(name: string): Promise<void> {
  await deleteSkillClient(name)
}

export async function importSkillFeature(inputPath: string, overrideName?: string): Promise<string> {
  return importSkillClient(inputPath, overrideName)
}

export async function exportSkillFeature(name: string, outputPath: string): Promise<void> {
  await exportSkillClient(name, outputPath)
}

export async function listWorkersFeature(): Promise<WorkerDefinition[]> {
  return listWorkersClient()
}

export async function saveWorkerFeature(definition: WorkerDefinition): Promise<void> {
  await saveWorkerClient(definition)
}

export async function deleteWorkerFeature(name: string): Promise<void> {
  await deleteWorkerClient(name)
}

export async function getGlobalRulesFeature(): Promise<string | null> {
  return getGlobalRulesClient()
}

export async function saveGlobalRulesFeature(content: string): Promise<void> {
  await saveGlobalRulesClient(content)
}

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
  type CapabilityPreset,
  type SkillDefinition,
  type WorkerDefinition,
} from '@/platform/tauri/clients/global-config-client'
import type {
  ApprovalMode,
  CapabilityMode,
  ClarificationMode,
} from '@/platform/tauri/clients/agent-engine-client'

export type {
  ApprovalMode,
  CapabilityMode,
  CapabilityPreset,
  ClarificationMode,
  SkillDefinition,
  WorkerDefinition,
}
export {
  BUILTIN_WORKER_TOOL_NAMES,
  WORKER_CAPABILITY_PRESETS,
  isBuiltinWorkerToolName,
  resolveWorkerVisibleTools,
  toolsForCapabilityPreset,
  type BuiltinWorkerToolName,
} from './worker-tool-contract'

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

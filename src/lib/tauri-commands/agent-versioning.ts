import type {
  Actor,
  EntityHead,
  FetchOpenAiModelsInput,
  OpenAiChatCompletionInput,
  OpenAiModelListResult,
  OpenAiProviderSettings,
  RecoverOutput,
  RollbackByCallIdInput,
  RollbackByRevisionInput,
  RollbackOutput,
  SaveOpenAiProviderSettingsInput,
} from '@/platform/tauri/clients/agent-versioning-client'
import {
  aiOpenAiChatCompletionClient,
  fetchOpenAiModelsClient,
  getOpenAiProviderSettingsClient,
  saveOpenAiProviderSettingsClient,
  vcGetCurrentHeadClient,
  vcRecoverClient,
  vcRollbackByCallIdClient,
  vcRollbackByRevisionClient,
} from '@/platform/tauri/clients/agent-versioning-client'

function normalizeOpenAiProviderSettings(input: OpenAiProviderSettings): OpenAiProviderSettings {
  const providerType = input.provider_type === 'openai' || input.provider_type === 'anthropic' || input.provider_type === 'gemini'
    ? input.provider_type
    : 'openai-compatible'
  const model = input.openai_model || 'gpt-4o-mini'
  const normalizedEnabledModels = Array.isArray(input.openai_enabled_models)
    ? Array.from(new Set(input.openai_enabled_models.map((item) => item.trim()).filter(Boolean)))
    : []
  const enabledModels = normalizedEnabledModels.length > 0 ? normalizedEnabledModels : [model]
  const embeddingModel = input.openai_embedding_model || model
  const detected = typeof input.openai_embedding_detected === 'boolean'
    ? input.openai_embedding_detected
    : enabledModels.includes(embeddingModel)
  const reason = (input.openai_embedding_detection_reason || '').trim()
    || (detected ? '' : 'embedding_model_unavailable')
  const enabled = Boolean(input.openai_embedding_enabled) && detected
  const planningEnabledModels = Array.isArray(input.planning_enabled_models)
    ? Array.from(new Set(input.planning_enabled_models.map((item) => item.trim()).filter(Boolean)))
    : []
  const planningModel = (input.planning_model || '').trim()
    || planningEnabledModels[0]
    || ''

  return {
    ...input,
    provider_type: providerType,
    openai_embedding_model: embeddingModel,
    openai_embedding_base_url: input.openai_embedding_base_url || input.openai_base_url,
    openai_embedding_api_key: input.openai_embedding_api_key || input.openai_api_key,
    openai_local_embedding_base_url: input.openai_local_embedding_base_url || 'http://127.0.0.1:11434/v1',
    openai_local_embedding_api_key: input.openai_local_embedding_api_key || '',
    openai_embedding_source: input.openai_embedding_source === 'local' ? 'local' : 'provider',
    openai_embedding_detected: detected,
    openai_embedding_detection_reason: reason,
    openai_embedding_enabled: enabled,
    openai_enabled_models: enabledModels,
    planning_generation_mode: input.planning_generation_mode === 'dedicated' ? 'dedicated' : 'follow_primary',
    planning_provider_type: input.planning_provider_type === 'openai'
      || input.planning_provider_type === 'anthropic'
      || input.planning_provider_type === 'gemini'
      ? input.planning_provider_type
      : 'openai-compatible',
    planning_base_url: input.planning_base_url || '',
    planning_api_key: input.planning_api_key || '',
    planning_model: planningModel,
    planning_enabled_models: planningEnabledModels.length > 0
      ? planningEnabledModels
      : (planningModel ? [planningModel] : []),
  }
}

export async function getOpenAiProviderSettings(): Promise<OpenAiProviderSettings> {
  const settings = await getOpenAiProviderSettingsClient()
  return normalizeOpenAiProviderSettings(settings)
}

export async function saveOpenAiProviderSettings(
  input: SaveOpenAiProviderSettingsInput,
): Promise<OpenAiProviderSettings> {
  const settings = await saveOpenAiProviderSettingsClient(input)
  return normalizeOpenAiProviderSettings(settings)
}

export async function fetchOpenAiModels(input: FetchOpenAiModelsInput): Promise<OpenAiModelListResult> {
  return fetchOpenAiModelsClient(input)
}

export async function aiOpenAiChatCompletion(input: OpenAiChatCompletionInput): Promise<unknown> {
  return aiOpenAiChatCompletionClient(input)
}

export async function vcGetCurrentHead(projectPath: string, entityId: string): Promise<EntityHead> {
  return vcGetCurrentHeadClient(projectPath, entityId)
}

export async function vcRollbackByRevision(input: RollbackByRevisionInput): Promise<RollbackOutput> {
  return vcRollbackByRevisionClient(input)
}

export async function vcRollbackByCallId(input: RollbackByCallIdInput): Promise<RollbackOutput> {
  return vcRollbackByCallIdClient(input)
}

export async function vcRecover(projectPath: string): Promise<RecoverOutput> {
  return vcRecoverClient(projectPath)
}

export type {
  Actor,
  EntityHead,
  FetchOpenAiModelsInput,
  OpenAiChatCompletionInput,
  OpenAiModelListResult,
  OpenAiProviderSettings,
  RecoverOutput,
  RollbackByCallIdInput,
  RollbackByRevisionInput,
  RollbackOutput,
  SaveOpenAiProviderSettingsInput,
}

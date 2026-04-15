import {
  fetchOpenAiModelsClient,
  getOpenAiProviderSettingsClient,
  saveOpenAiProviderSettingsClient,
  type OpenAiProviderSettings,
} from '@/platform/tauri/clients'
import {
  fetchOpenAiModels,
  getOpenAiProviderSettings,
  saveOpenAiProviderSettings,
  scanProjectsDirectory,
} from '@/lib/tauri-commands'

function normalizeProviderSettings(input: OpenAiProviderSettings): OpenAiProviderSettings {
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

export async function loadOpenAiProviderSettingsFeature() {
  try {
    const loaded = await getOpenAiProviderSettingsClient()
    return normalizeProviderSettings(loaded)
  } catch {
    const loaded = await getOpenAiProviderSettings()
    return normalizeProviderSettings(loaded)
  }
}

export async function saveOpenAiProviderSettingsFeature(input: {
  provider_type?: 'openai' | 'anthropic' | 'gemini' | 'openai-compatible'
  openai_base_url: string
  openai_api_key: string
  openai_model?: string
  openai_embedding_model?: string
  openai_embedding_base_url?: string
  openai_embedding_api_key?: string
  openai_local_embedding_base_url?: string
  openai_local_embedding_api_key?: string
  openai_embedding_source?: 'provider' | 'local'
  openai_embedding_enabled?: boolean
  openai_embedding_detected?: boolean
  openai_embedding_detection_reason?: string
  openai_enabled_models: string[]
  planning_generation_mode?: 'follow_primary' | 'dedicated'
  planning_provider_type?: 'openai' | 'anthropic' | 'gemini' | 'openai-compatible'
  planning_base_url?: string
  planning_api_key?: string
  planning_model?: string
  planning_enabled_models?: string[]
}) {
  try {
    const saved = await saveOpenAiProviderSettingsClient(input)
    return normalizeProviderSettings(saved)
  } catch {
    const saved = await saveOpenAiProviderSettings(input)
    return normalizeProviderSettings(saved as OpenAiProviderSettings)
  }
}

export async function fetchOpenAiModelsFeature(input: {
  openai_base_url: string
  openai_api_key: string
}) {
  try {
    return await fetchOpenAiModelsClient(input)
  } catch {
    return fetchOpenAiModels(input)
  }
}

export async function scanProjectsForSettings(rootDir: string) {
  return scanProjectsDirectory(rootDir)
}

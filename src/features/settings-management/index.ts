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
  const model = input.openai_model || 'gpt-4o-mini'
  const enabledModels = Array.isArray(input.openai_enabled_models) ? input.openai_enabled_models : [model]
  const embeddingModel = input.openai_embedding_model || model
  const detected = typeof input.openai_embedding_detected === 'boolean'
    ? input.openai_embedding_detected
    : enabledModels.includes(embeddingModel)
  const reason = (input.openai_embedding_detection_reason || '').trim()
    || (detected ? '' : 'embedding_model_unavailable')
  const enabled = Boolean(input.openai_embedding_enabled) && detected

  return {
    ...input,
    openai_embedding_model: embeddingModel,
    openai_embedding_base_url: input.openai_embedding_base_url || input.openai_base_url,
    openai_embedding_api_key: input.openai_embedding_api_key || input.openai_api_key,
    openai_local_embedding_base_url: input.openai_local_embedding_base_url || 'http://127.0.0.1:11434/v1',
    openai_local_embedding_api_key: input.openai_local_embedding_api_key || '',
    openai_embedding_source: input.openai_embedding_source === 'local' ? 'local' : 'provider',
    openai_embedding_detected: detected,
    openai_embedding_detection_reason: reason,
    openai_embedding_enabled: enabled,
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

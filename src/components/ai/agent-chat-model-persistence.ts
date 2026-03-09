import { saveAgentProviderSettings } from '@/features/agent-chat'
import { useSettingsStore } from '@/state/settings'

export async function persistSelectedChatModel(model: string) {
  const state = useSettingsStore.getState()

  const saved = await saveAgentProviderSettings({
    openai_base_url: state.openaiBaseUrl,
    openai_api_key: state.openaiApiKey,
    openai_model: model,
    openai_embedding_model: state.openaiEmbeddingModel,
    openai_embedding_base_url: state.openaiEmbeddingBaseUrl,
    openai_embedding_api_key: state.openaiEmbeddingApiKey,
    openai_local_embedding_base_url: state.openaiLocalEmbeddingBaseUrl,
    openai_local_embedding_api_key: state.openaiLocalEmbeddingApiKey,
    openai_embedding_source: state.openaiEmbeddingSource,
    openai_embedding_enabled: state.openaiEmbeddingEnabled,
    openai_embedding_detected: state.openaiEmbeddingDetected,
    openai_embedding_detection_reason: state.openaiEmbeddingDetectionReason,
    openai_enabled_models: state.openaiEnabledModels,
  })

  state.setOpenAiProviderSettings({
    baseUrl: saved.openai_base_url,
    apiKey: saved.openai_api_key,
    model: saved.openai_model,
    embeddingModel: saved.openai_embedding_model,
    embeddingBaseUrl: saved.openai_embedding_base_url,
    embeddingApiKey: saved.openai_embedding_api_key,
    localEmbeddingBaseUrl: saved.openai_local_embedding_base_url,
    localEmbeddingApiKey: saved.openai_local_embedding_api_key,
    embeddingSource: saved.openai_embedding_source,
    embeddingEnabled: saved.openai_embedding_enabled,
    embeddingAvailability: {
      enabled: saved.openai_embedding_enabled,
      detected: saved.openai_embedding_detected,
      reason: saved.openai_embedding_detection_reason,
    },
    enabledModels: saved.openai_enabled_models,
  })
}

export async function persistSelectedEmbeddingModel(model: string) {
  const state = useSettingsStore.getState()

  const saved = await saveAgentProviderSettings({
    openai_base_url: state.openaiBaseUrl,
    openai_api_key: state.openaiApiKey,
    openai_model: state.openaiModel,
    openai_embedding_model: model,
    openai_embedding_base_url: state.openaiEmbeddingBaseUrl,
    openai_embedding_api_key: state.openaiEmbeddingApiKey,
    openai_local_embedding_base_url: state.openaiLocalEmbeddingBaseUrl,
    openai_local_embedding_api_key: state.openaiLocalEmbeddingApiKey,
    openai_embedding_source: state.openaiEmbeddingSource,
    openai_embedding_enabled: state.openaiEmbeddingEnabled,
    openai_embedding_detected: state.openaiEmbeddingDetected,
    openai_embedding_detection_reason: state.openaiEmbeddingDetectionReason,
    openai_enabled_models: state.openaiEnabledModels,
  })

  state.setOpenAiProviderSettings({
    baseUrl: saved.openai_base_url,
    apiKey: saved.openai_api_key,
    model: saved.openai_model,
    embeddingModel: saved.openai_embedding_model,
    embeddingBaseUrl: saved.openai_embedding_base_url,
    embeddingApiKey: saved.openai_embedding_api_key,
    localEmbeddingBaseUrl: saved.openai_local_embedding_base_url,
    localEmbeddingApiKey: saved.openai_local_embedding_api_key,
    embeddingSource: saved.openai_embedding_source,
    embeddingEnabled: saved.openai_embedding_enabled,
    embeddingAvailability: {
      enabled: saved.openai_embedding_enabled,
      detected: saved.openai_embedding_detected,
      reason: saved.openai_embedding_detection_reason,
    },
    enabledModels: saved.openai_enabled_models,
  })
}

export async function persistSelectedEmbeddingSource(source: 'provider' | 'local') {
  const state = useSettingsStore.getState()

  const saved = await saveAgentProviderSettings({
    openai_base_url: state.openaiBaseUrl,
    openai_api_key: state.openaiApiKey,
    openai_model: state.openaiModel,
    openai_embedding_model: state.openaiEmbeddingModel,
    openai_embedding_base_url: state.openaiEmbeddingBaseUrl,
    openai_embedding_api_key: state.openaiEmbeddingApiKey,
    openai_local_embedding_base_url: state.openaiLocalEmbeddingBaseUrl,
    openai_local_embedding_api_key: state.openaiLocalEmbeddingApiKey,
    openai_embedding_source: source,
    openai_embedding_enabled: state.openaiEmbeddingEnabled,
    openai_embedding_detected: state.openaiEmbeddingDetected,
    openai_embedding_detection_reason: state.openaiEmbeddingDetectionReason,
    openai_enabled_models: state.openaiEnabledModels,
  })

  state.setOpenAiProviderSettings({
    baseUrl: saved.openai_base_url,
    apiKey: saved.openai_api_key,
    model: saved.openai_model,
    embeddingModel: saved.openai_embedding_model,
    embeddingBaseUrl: saved.openai_embedding_base_url,
    embeddingApiKey: saved.openai_embedding_api_key,
    localEmbeddingBaseUrl: saved.openai_local_embedding_base_url,
    localEmbeddingApiKey: saved.openai_local_embedding_api_key,
    embeddingSource: saved.openai_embedding_source,
    embeddingEnabled: saved.openai_embedding_enabled,
    embeddingAvailability: {
      enabled: saved.openai_embedding_enabled,
      detected: saved.openai_embedding_detected,
      reason: saved.openai_embedding_detection_reason,
    },
    enabledModels: saved.openai_enabled_models,
  })
}

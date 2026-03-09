import { fetchOpenAiModelsFeature } from '@/features/settings-management'

import type { TempState } from './use-settings-dialog-controller'

export function syncProviderDraftFromStore(input: {
  settings: {
    openaiBaseUrl: string
    openaiApiKey: string
    openaiModel: string
    openaiEmbeddingModel: string
    openaiEmbeddingBaseUrl: string
    openaiEmbeddingApiKey: string
    openaiLocalEmbeddingBaseUrl: string
    openaiLocalEmbeddingApiKey: string
    openaiEmbeddingSource: 'provider' | 'local'
    openaiEmbeddingEnabled: boolean
    openaiEmbeddingDetected: boolean
    openaiEmbeddingDetectionReason?: string
    openaiEnabledModels: string[]
  }
  temp: TempState
}) {
  input.temp.setTempOpenAiBaseUrl(input.settings.openaiBaseUrl)
  input.temp.setTempOpenAiApiKey(input.settings.openaiApiKey)
  input.temp.setTempOpenAiModel(input.settings.openaiModel)
  input.temp.setTempOpenAiEmbeddingModel(input.settings.openaiEmbeddingModel || input.settings.openaiModel)
  input.temp.setTempOpenAiEmbeddingBaseUrl(input.settings.openaiEmbeddingBaseUrl || input.settings.openaiBaseUrl)
  input.temp.setTempOpenAiEmbeddingApiKey(input.settings.openaiEmbeddingApiKey || input.settings.openaiApiKey)
  input.temp.setTempOpenAiLocalEmbeddingBaseUrl(input.settings.openaiLocalEmbeddingBaseUrl || 'http://127.0.0.1:11434/v1')
  input.temp.setTempOpenAiLocalEmbeddingApiKey(input.settings.openaiLocalEmbeddingApiKey || '')
  input.temp.setTempOpenAiEmbeddingSource(input.settings.openaiEmbeddingSource || 'provider')
  input.temp.setTempOpenAiEmbeddingEnabled(Boolean(input.settings.openaiEmbeddingEnabled) && Boolean(input.settings.openaiEmbeddingDetected))
  input.temp.setTempOpenAiEmbeddingDetected(Boolean(input.settings.openaiEmbeddingDetected))
  input.temp.setTempOpenAiEmbeddingDetectionReason(input.settings.openaiEmbeddingDetectionReason || '')
  input.temp.setTempOpenAiEnabledModels(input.settings.openaiEnabledModels)
  input.temp.setTempFetchedModels([])
  input.temp.setTempFetchingModels(false)
  input.temp.setTempModelFetchError('')
}

function normalizeModelList(models: string[]) {
  const seen = new Set<string>()
  const normalized: string[] = []

  for (const raw of models) {
    const value = String(raw ?? '').trim()
    if (!value || seen.has(value)) continue
    seen.add(value)
    normalized.push(value)
  }

  return normalized
}

function resolveVisibleModels(input: {
  fetchedModels: string[]
  previousEnabledModels: string[]
  preferredModel: string
}) {
  const fetched = normalizeModelList(input.fetchedModels)
  const preferred = (input.preferredModel || '').trim()

  const nextEnabled = normalizeModelList(input.previousEnabledModels).filter((model) =>
    fetched.includes(model),
  )

  if (nextEnabled.length === 0) {
    if (preferred && fetched.includes(preferred)) {
      nextEnabled.push(preferred)
    } else if (fetched.length > 0) {
      nextEnabled.push(fetched[0])
    }
  }

  const nextSelected = preferred && fetched.includes(preferred)
    ? preferred
    : (nextEnabled[0] || fetched[0] || '')

  return {
    fetched,
    enabled: nextEnabled,
    selected: nextSelected,
  }
}

export async function fetchProviderModelsAction(temp: TempState) {
  try {
    temp.setTempFetchingModels(true)
    temp.setTempModelFetchError('')

    const result = await fetchOpenAiModelsFeature({
      openai_base_url: temp.tempOpenAiBaseUrl.trim(),
      openai_api_key: temp.tempOpenAiApiKey.trim(),
    })

    const resolved = resolveVisibleModels({
      fetchedModels: result.models,
      previousEnabledModels: temp.tempOpenAiEnabledModels,
      preferredModel: temp.tempOpenAiModel,
    })

    if (resolved.fetched.length === 0) {
      temp.setTempFetchedModels([])
      temp.setTempOpenAiEnabledModels([])
      temp.setTempOpenAiModel('')
      temp.setTempOpenAiEmbeddingDetected(false)
      temp.setTempOpenAiEmbeddingEnabled(false)
      temp.setTempOpenAiEmbeddingDetectionReason('embedding_model_unavailable')
      temp.setTempModelFetchError('E_AI_MODELS_EMPTY')
      return
    }

    temp.setTempFetchedModels(resolved.fetched)
    temp.setTempOpenAiEnabledModels(resolved.enabled)
    temp.setTempOpenAiModel(resolved.selected)

    const embeddingDetected = resolved.fetched.includes(temp.tempOpenAiEmbeddingModel)
    temp.setTempOpenAiEmbeddingDetected(embeddingDetected)
    temp.setTempOpenAiEmbeddingDetectionReason(embeddingDetected ? '' : 'embedding_model_unavailable')
    if (!embeddingDetected) {
      temp.setTempOpenAiEmbeddingEnabled(false)
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    temp.setTempModelFetchError(message)
  } finally {
    temp.setTempFetchingModels(false)
  }
}

export async function fetchEmbeddingModelsAction(temp: TempState) {
  try {
    temp.setTempFetchingModels(true)
    temp.setTempModelFetchError('')

    const useLocal = temp.tempOpenAiEmbeddingSource === 'local'
    const result = await fetchOpenAiModelsFeature({
      openai_base_url: useLocal ? temp.tempOpenAiLocalEmbeddingBaseUrl.trim() : temp.tempOpenAiEmbeddingBaseUrl.trim(),
      openai_api_key: useLocal ? temp.tempOpenAiLocalEmbeddingApiKey.trim() : temp.tempOpenAiEmbeddingApiKey.trim(),
    })

    const fetched = normalizeModelList(result.models)

    if (fetched.length === 0) {
      temp.setTempFetchedModels([])
      temp.setTempOpenAiEnabledModels([])
      temp.setTempOpenAiEmbeddingDetected(false)
      temp.setTempOpenAiEmbeddingEnabled(false)
      temp.setTempOpenAiEmbeddingDetectionReason('embedding_model_unavailable')
      temp.setTempModelFetchError('E_AI_MODELS_EMPTY')
      return
    }

    const nextModels = normalizeModelList([...temp.tempOpenAiEnabledModels, ...fetched])
    temp.setTempFetchedModels(fetched)
    temp.setTempOpenAiEnabledModels(nextModels)

    const detected = nextModels.includes(temp.tempOpenAiEmbeddingModel)
    temp.setTempOpenAiEmbeddingDetected(detected)
    temp.setTempOpenAiEmbeddingDetectionReason(detected ? '' : 'embedding_model_unavailable')

    if (!detected) {
      temp.setTempOpenAiEmbeddingEnabled(false)
      const fallbackEmbedding = nextModels[0] || temp.tempOpenAiModel || ''
      temp.setTempOpenAiEmbeddingModel(fallbackEmbedding)
      const fallbackDetected = Boolean(fallbackEmbedding) && nextModels.includes(fallbackEmbedding)
      temp.setTempOpenAiEmbeddingDetected(fallbackDetected)
      temp.setTempOpenAiEmbeddingDetectionReason(fallbackDetected ? '' : 'embedding_model_unavailable')
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    temp.setTempModelFetchError(message)
  } finally {
    temp.setTempFetchingModels(false)
  }
}

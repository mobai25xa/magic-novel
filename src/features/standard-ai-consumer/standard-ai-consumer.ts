import { useMemo } from 'react'

import { persistSelectedChatModel } from '@/components/ai/agent-chat-model-persistence'
import { useSettingsStore, type ProviderType } from '@/state/settings'
import type { SettingsState } from '@/stores/settings-types'

const DEFAULT_STANDARD_AI_MODEL = 'gpt-4o-mini'
const DEFAULT_STANDARD_AI_PROVIDER = 'openai-compatible'

type StandardAiModelSettings = Pick<SettingsState, 'openaiEnabledModels' | 'openaiModel'>
type StandardAiProviderSettings = Pick<
  SettingsState,
  'providerType' | 'openaiEnabledModels' | 'openaiModel' | 'openaiBaseUrl' | 'openaiApiKey'
>

export type StandardAiProviderConfig = {
  provider: string
  model?: string
  base_url?: string
  api_key?: string
}

export type StandardAiProviderInput = {
  client_request_id?: string
  provider?: string
  model?: string
  base_url?: string
  api_key?: string
}

export type StandardAiProviderConfigIssue = {
  code: 'E_AI_PROVIDER_BASE_URL_MISSING' | 'E_AI_PROVIDER_API_KEY_MISSING'
  message: string
}

export type StandardAiProviderConfigIssueMessages = {
  missingBaseUrl: string
  missingApiKey: string
}

export type StandardAiConsumerState = {
  availableModels: string[]
  selectedModel: string
  handleSelectModel: (model: string) => void
  providerConfig: StandardAiProviderConfig
}

function asNonEmptyString(input?: string | null) {
  if (typeof input !== 'string') {
    return undefined
  }

  const normalized = input.trim()
  return normalized ? normalized : undefined
}

function normalizeModels(input: string[]) {
  const seen = new Set<string>()

  return (input || [])
    .map((item) => String(item || '').trim())
    .filter((item) => {
      if (!item || seen.has(item)) {
        return false
      }

      seen.add(item)
      return true
    })
}

function resolveStandardAiProvider(
  _providerType?: ProviderType,
  overrideProvider?: string,
) {
  const explicitProvider = asNonEmptyString(overrideProvider)
  if (explicitProvider) {
    return explicitProvider
  }

  // V1 keeps the editor panel request shape stable until multi-provider
  // routing is fully standardized across all consumers.
  return DEFAULT_STANDARD_AI_PROVIDER
}

export function resolveStandardAiAvailableModels(settings: StandardAiModelSettings) {
  const normalizedModels = normalizeModels(settings.openaiEnabledModels)
  if (normalizedModels.length > 0) {
    return normalizedModels
  }

  return [asNonEmptyString(settings.openaiModel) ?? DEFAULT_STANDARD_AI_MODEL]
}

export function resolveStandardAiSelectedModel(
  settings: StandardAiModelSettings,
  overrideModel?: string,
) {
  const availableModels = resolveStandardAiAvailableModels(settings)
  const requestedModel = asNonEmptyString(overrideModel) ?? asNonEmptyString(settings.openaiModel)

  if (requestedModel && availableModels.includes(requestedModel)) {
    return requestedModel
  }

  return availableModels[0]
}

export function buildStandardAiProviderConfig(input: {
  settings?: StandardAiProviderSettings
  model?: string
  provider?: string
} = {}): StandardAiProviderConfig {
  const settings = input.settings ?? useSettingsStore.getState()

  return {
    provider: resolveStandardAiProvider(settings.providerType, input.provider),
    model: resolveStandardAiSelectedModel(settings, input.model),
    base_url: asNonEmptyString(settings.openaiBaseUrl),
    api_key: asNonEmptyString(settings.openaiApiKey),
  }
}

export function buildStandardAiProviderInput(input: {
  settings?: StandardAiProviderSettings
  clientRequestId?: string
  model?: string
  provider?: string
} = {}): StandardAiProviderInput {
  const clientRequestId = asNonEmptyString(input.clientRequestId)
  const providerConfig = buildStandardAiProviderConfig(input)

  if (!clientRequestId) {
    return providerConfig
  }

  return {
    client_request_id: clientRequestId,
    ...providerConfig,
  }
}

export function getStandardAiProviderConfigIssue(input: {
  providerConfig: StandardAiProviderConfig
  messages: StandardAiProviderConfigIssueMessages
}): StandardAiProviderConfigIssue | null {
  if (!asNonEmptyString(input.providerConfig.base_url)) {
    return {
      code: 'E_AI_PROVIDER_BASE_URL_MISSING',
      message: input.messages.missingBaseUrl,
    }
  }

  if (!asNonEmptyString(input.providerConfig.api_key)) {
    return {
      code: 'E_AI_PROVIDER_API_KEY_MISSING',
      message: input.messages.missingApiKey,
    }
  }

  return null
}

export function useStandardAiConsumerState(): StandardAiConsumerState {
  const providerType = useSettingsStore((state) => state.providerType)
  const openaiEnabledModels = useSettingsStore((state) => state.openaiEnabledModels)
  const openaiModel = useSettingsStore((state) => state.openaiModel)
  const openaiBaseUrl = useSettingsStore((state) => state.openaiBaseUrl)
  const openaiApiKey = useSettingsStore((state) => state.openaiApiKey)
  const setOpenAiModel = useSettingsStore((state) => state.setOpenAiModel)

  const availableModels = useMemo(
    () => resolveStandardAiAvailableModels({ openaiEnabledModels, openaiModel }),
    [openaiEnabledModels, openaiModel],
  )

  const selectedModel = useMemo(
    () => resolveStandardAiSelectedModel({ openaiEnabledModels, openaiModel }),
    [openaiEnabledModels, openaiModel],
  )

  const providerConfig = useMemo(
    () => buildStandardAiProviderConfig({
      settings: {
        providerType,
        openaiEnabledModels,
        openaiModel,
        openaiBaseUrl,
        openaiApiKey,
      },
      model: selectedModel,
    }),
    [providerType, openaiEnabledModels, openaiModel, openaiBaseUrl, openaiApiKey, selectedModel],
  )

  const handleSelectModel = (model: string) => {
    setOpenAiModel(model)

    void persistSelectedChatModel(model).catch((error) => {
      console.error('Failed to persist selected model:', error)
    })
  }

  return {
    availableModels,
    selectedModel,
    handleSelectModel,
    providerConfig,
  }
}

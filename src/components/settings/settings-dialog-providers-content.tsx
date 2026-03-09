import { Input, Toggle } from '@/magic-ui/components'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'
import { PROVIDER_CONFIGS } from '@/stores/settings-constants'
import type { ProviderType } from '@/stores/settings-types'

import { ProviderModelSelector } from './provider-model-selector'
import type { SettingsDialogTranslations } from './settings-dialog-types'
import type { TempState } from './use-settings-dialog-controller'

const PROVIDER_TYPE_OPTIONS: { value: ProviderType; labelKey: string }[] = [
  { value: 'openai', labelKey: 'providerTypeOpenAi' },
  { value: 'anthropic', labelKey: 'providerTypeAnthropic' },
  { value: 'gemini', labelKey: 'providerTypeGemini' },
  { value: 'openai-compatible', labelKey: 'providerTypeOpenAiCompatible' },
]

function getProviderConfig(providerType: ProviderType) {
  return PROVIDER_CONFIGS[providerType] || PROVIDER_CONFIGS['openai-compatible']
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

function getVisibleProviderModels(temp: TempState) {
  const fetched = normalizeModelList(temp.tempFetchedModels)
  const enabled = normalizeModelList(temp.tempOpenAiEnabledModels)

  if (fetched.length === 0) {
    if (enabled.length > 0) return enabled
    const fallback = String(temp.tempOpenAiModel || '').trim()
    return fallback ? [fallback] : []
  }

  const visible = enabled.filter((model) => fetched.includes(model))
  if (visible.length > 0) return visible

  const fallback = [temp.tempOpenAiModel, fetched[0]].find((value) => value && fetched.includes(value))
  return fallback ? [fallback] : []
}

function renderModelItems(models: string[]) {
  return models.map((model) => (
    <SelectItem key={model} value={model}>{model}</SelectItem>
  ))
}

function renderProviderCredentials(translations: SettingsDialogTranslations, temp: TempState) {
  const config = getProviderConfig(temp.tempProviderType)
  const visibleModels = getVisibleProviderModels(temp)

  return (
    <>
      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerBaseUrl}</div>
        <Input
          value={temp.tempOpenAiBaseUrl}
          onChange={(e) => temp.setTempOpenAiBaseUrl(e.target.value)}
          placeholder={config.baseUrlPlaceholder}
        />
      </div>
      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerApiKey}</div>
        <Input
          type="password"
          value={temp.tempOpenAiApiKey}
          onChange={(e) => temp.setTempOpenAiApiKey(e.target.value)}
          placeholder={config.apiKeyPlaceholder}
        />
      </div>
      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerModel}</div>
        <Select
          value={temp.tempOpenAiModel || ''}
          onValueChange={temp.setTempOpenAiModel}
          disabled={visibleModels.length === 0}
        >
          <SelectTrigger className="w-full">
            <SelectValue placeholder={translations.settings.providerNoModels} />
          </SelectTrigger>
          <SelectContent>{renderModelItems(visibleModels)}</SelectContent>
        </Select>
      </div>
    </>
  )
}

function renderEmbeddingSelector(translations: SettingsDialogTranslations, temp: TempState) {
  const source = temp.tempOpenAiEmbeddingSource
  const visibleModels = getVisibleProviderModels(temp)

  return (
    <>
      <div className="provider-card-disabled">
        <div className="flex items-center justify-between">
          <div>
            <h5 className="text-sm font-medium">{translations.settings.providerEmbeddingEnabled || 'Enable Embedding Search'}</h5>
            {!temp.tempOpenAiEmbeddingDetected ? (
              <p className="text-xs text-warning mt-1">{translations.settings.providerEmbeddingUnavailable || 'No embedding model detected. Semantic retrieval is disabled.'}</p>
            ) : null}
          </div>
          <Toggle
            checked={temp.tempOpenAiEmbeddingEnabled}
            disabled={!temp.tempOpenAiEmbeddingDetected}
            onChange={(e) => temp.setTempOpenAiEmbeddingEnabled(e.target.checked)}
          />
        </div>
      </div>

      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerEmbeddingSource || 'Embedding Source'}</div>
        <Select value={source} onValueChange={(value) => temp.setTempOpenAiEmbeddingSource(value as 'provider' | 'local')}>
          <SelectTrigger className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="provider">{translations.settings.providerEmbeddingSourceProvider || 'Provider'}</SelectItem>
            <SelectItem value="local">{translations.settings.providerEmbeddingSourceLocal || 'Local'}</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerEmbeddingModel || 'Embedding Model'}</div>
        <Select
          value={temp.tempOpenAiEmbeddingModel || temp.tempOpenAiModel || ''}
          onValueChange={(value) => {
            temp.setTempOpenAiEmbeddingModel(value)
            const detected = visibleModels.includes(value)
            temp.setTempOpenAiEmbeddingDetected(detected)
            temp.setTempOpenAiEmbeddingDetectionReason(detected ? '' : 'embedding_model_unavailable')
            if (!detected) {
              temp.setTempOpenAiEmbeddingEnabled(false)
            }
          }}
          disabled={visibleModels.length === 0}
        >
          <SelectTrigger className="w-full">
            <SelectValue placeholder={translations.settings.providerNoModels} />
          </SelectTrigger>
          <SelectContent>{renderModelItems(visibleModels)}</SelectContent>
        </Select>
      </div>
    </>
  )
}

function renderEmbeddingEndpointSettings(translations: SettingsDialogTranslations, temp: TempState) {
  if (temp.tempOpenAiEmbeddingSource === 'provider') {
    return (
      <>
        <div className="space-y-2">
          <div className="text-xs text-muted-foreground">{translations.settings.providerEmbeddingBaseUrl || 'Embedding Base URL'}</div>
          <Input
            value={temp.tempOpenAiEmbeddingBaseUrl}
            onChange={(e) => temp.setTempOpenAiEmbeddingBaseUrl(e.target.value)}
            placeholder="https://api.openai.com/v1"
          />
        </div>
        <div className="space-y-2">
          <div className="text-xs text-muted-foreground">{translations.settings.providerEmbeddingApiKey || 'Embedding API Key'}</div>
          <Input
            type="password"
            value={temp.tempOpenAiEmbeddingApiKey}
            onChange={(e) => temp.setTempOpenAiEmbeddingApiKey(e.target.value)}
            placeholder="sk-..."
          />
        </div>
      </>
    )
  }

  return (
    <>
      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerLocalEmbeddingBaseUrl || 'Local Embedding Base URL'}</div>
        <Input
          value={temp.tempOpenAiLocalEmbeddingBaseUrl}
          onChange={(e) => temp.setTempOpenAiLocalEmbeddingBaseUrl(e.target.value)}
          placeholder="http://127.0.0.1:11434/v1"
        />
      </div>
      <div className="space-y-2">
        <div className="text-xs text-muted-foreground">{translations.settings.providerLocalEmbeddingApiKey || 'Local Embedding API Key (optional)'}</div>
        <Input
          type="password"
          value={temp.tempOpenAiLocalEmbeddingApiKey}
          onChange={(e) => temp.setTempOpenAiLocalEmbeddingApiKey(e.target.value)}
          placeholder="optional"
        />
      </div>
    </>
  )
}

function handleProviderTypeChange(temp: TempState, nextType: ProviderType) {
  const config = getProviderConfig(nextType)
  temp.setTempProviderType(nextType)

  if (config.defaultBaseUrl && !temp.tempOpenAiBaseUrl.trim()) {
    temp.setTempOpenAiBaseUrl(config.defaultBaseUrl)
  }

  if (config.presetModels.length > 0 && temp.tempOpenAiEnabledModels.length <= 1) {
    temp.setTempOpenAiEnabledModels(config.presetModels)
    if (!config.presetModels.includes(temp.tempOpenAiModel)) {
      temp.setTempOpenAiModel(config.presetModels[0])
    }
    temp.setTempFetchedModels(config.presetModels)
  }
}

function renderProviderTypeSelector(translations: SettingsDialogTranslations, temp: TempState) {
  return (
    <div className="space-y-2">
      <div className="text-xs text-muted-foreground">{translations.settings.providerType}</div>
      <Select
        value={temp.tempProviderType}
        onValueChange={(value) => handleProviderTypeChange(temp, value as ProviderType)}
      >
        <SelectTrigger className="w-full">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {PROVIDER_TYPE_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              {translations.settings[opt.labelKey] || opt.value}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  )
}

function renderLlmServiceSection(
  translations: SettingsDialogTranslations,
  temp: TempState,
  onFetchChatModels: () => Promise<void>,
) {
  return (
    <div className="settings-section space-y-3">
      <div>
        <h4 className="text-sm font-medium">{translations.settings.providers}</h4>
        <p className="text-xs text-muted-foreground">{translations.settings.providerDescription}</p>
      </div>
      {renderProviderTypeSelector(translations, temp)}
      {renderProviderCredentials(translations, temp)}
      <ProviderModelSelector
        title={translations.settings.providerEmbeddingForChat || 'Chat Models'}
        translations={translations}
        candidateModels={temp.tempFetchedModels}
        enabledModels={temp.tempOpenAiEnabledModels}
        selectedModel={temp.tempOpenAiModel}
        onSelectModel={temp.setTempOpenAiModel}
        onEnabledModelsChange={(models) => temp.setTempOpenAiEnabledModels(models)}
        tempFetchingModels={temp.tempFetchingModels}
        tempModelFetchError={temp.tempModelFetchError}
        onFetchModels={onFetchChatModels}
      />
    </div>
  )
}

function renderEmbeddingServiceSection(
  translations: SettingsDialogTranslations,
  temp: TempState,
  onFetchEmbeddingModels: () => Promise<void>,
) {
  return (
    <div className="settings-section space-y-3">
      <div>
        <h4 className="text-sm font-medium">{translations.settings.providerEmbeddingType || 'Embedding Service'}</h4>
        <p className="text-xs text-muted-foreground">
          {translations.settings.providerEmbeddingDescription || translations.settings.providerDescription}
        </p>
      </div>
      {renderEmbeddingSelector(translations, temp)}
      {renderEmbeddingEndpointSettings(translations, temp)}
      <ProviderModelSelector
        title={translations.settings.providerEmbeddingForSearch || 'Embedding Models'}
        translations={translations}
        candidateModels={temp.tempFetchedModels}
        enabledModels={temp.tempOpenAiEnabledModels}
        selectedModel={temp.tempOpenAiEmbeddingModel}
        onSelectModel={(model) => {
          temp.setTempOpenAiEmbeddingModel(model)
          const detected = temp.tempOpenAiEnabledModels.includes(model)
          temp.setTempOpenAiEmbeddingDetected(detected)
          temp.setTempOpenAiEmbeddingDetectionReason(detected ? '' : 'embedding_model_unavailable')
          if (!detected) {
            temp.setTempOpenAiEmbeddingEnabled(false)
          }
        }}
        onEnabledModelsChange={(models) => {
          temp.setTempOpenAiEnabledModels(models)
          const detected = models.includes(temp.tempOpenAiEmbeddingModel)
          temp.setTempOpenAiEmbeddingDetected(detected)
          temp.setTempOpenAiEmbeddingDetectionReason(detected ? '' : 'embedding_model_unavailable')
          if (!detected) {
            temp.setTempOpenAiEmbeddingEnabled(false)
          }
        }}
        tempFetchingModels={temp.tempFetchingModels}
        tempModelFetchError={temp.tempModelFetchError}
        onFetchModels={onFetchEmbeddingModels}
      />
    </div>
  )
}

export function renderProvidersContent(
  translations: SettingsDialogTranslations,
  temp: TempState,
  onFetchChatModels: () => Promise<void>,
  onFetchEmbeddingModels: () => Promise<void>,
) {
  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-2">{translations.settings.providers}</h3>
        <p className="text-sm text-muted-foreground">{translations.settings.providersDescription}</p>
      </div>

      {renderLlmServiceSection(translations, temp, onFetchChatModels)}
      {renderEmbeddingServiceSection(translations, temp, onFetchEmbeddingModels)}
    </div>
  )
}

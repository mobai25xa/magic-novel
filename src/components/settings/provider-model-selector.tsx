import { RotateCw } from 'lucide-react'

import { Button } from '@/magic-ui/components'
import type { SettingsDialogTranslations } from './settings-dialog-types'

type ProviderModelSelectorProps = {
  title: string
  translations: SettingsDialogTranslations
  candidateModels: string[]
  enabledModels: string[]
  selectedModel: string
  onSelectModel: (model: string) => void
  onEnabledModelsChange: (models: string[]) => void
  tempFetchingModels: boolean
  tempModelFetchError: string
  onFetchModels: () => Promise<void>
}

function parseFetchError(input: string) {
  const match = input.match(/E_[A-Z_]+/)
  if (match) return match[0]
  return input
}

function buildNextEnabledModels(input: {
  previous: string[]
  model: string
  checked: boolean
}) {
  if (input.checked) {
    if (input.previous.includes(input.model)) return input.previous
    return [...input.previous, input.model]
  }

  if (!input.previous.includes(input.model)) return input.previous
  if (input.previous.length <= 1) return input.previous
  return input.previous.filter((item) => item !== input.model)
}

function buildNextSelectedModel(input: {
  previousSelectedModel: string
  model: string
  nextEnabledModels: string[]
}) {
  if (!input.previousSelectedModel) return input.nextEnabledModels[0] || ''
  if (input.nextEnabledModels.includes(input.previousSelectedModel)) {
    return input.previousSelectedModel
  }
  if (input.nextEnabledModels.includes(input.model)) {
    return input.model
  }
  return input.nextEnabledModels[0] || ''
}

export function ProviderModelSelector(input: ProviderModelSelectorProps) {
  return (
    <div className="settings-section space-y-3">
      <div className="flex items-center justify-between gap-3">
        <div>
          <h4 className="text-sm font-medium">{input.title}</h4>
          <p className="text-xs text-muted-foreground">{input.translations.settings.providerEnabledModelsDescription}</p>
        </div>
        <Button
          type="button"
          variant="settingsOutline"
          size="icon"
          onClick={() => {
            void input.onFetchModels()
          }}
          title={input.translations.settings.providerRefreshModels}
          aria-label={input.translations.settings.providerRefreshModels}
          className="h-8 w-8"
        >
          <RotateCw className={`h-4 w-4 text-muted-foreground ${input.tempFetchingModels ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {input.tempModelFetchError ? (
        <div className="text-xs text-destructive">{parseFetchError(input.tempModelFetchError)}</div>
      ) : null}

      <div className="max-h-44 overflow-y-auto settings-section p-2 space-y-1">
        {input.candidateModels.length > 0 ? (
          input.candidateModels.map((model) => {
            const checked = input.enabledModels.includes(model)
            const isOnlyOne = checked && input.enabledModels.length <= 1
            return (
              <label key={model} className="provider-model-row">
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(event) => {
                    const nextEnabledModels = buildNextEnabledModels({
                      previous: input.enabledModels,
                      model,
                      checked: event.target.checked,
                    })

                    input.onEnabledModelsChange(nextEnabledModels)
                    input.onSelectModel(
                      buildNextSelectedModel({
                        previousSelectedModel: input.selectedModel,
                        model,
                        nextEnabledModels,
                      }),
                    )
                  }}
                  disabled={isOnlyOne}
                  className="provider-model-checkbox"
                />
                <span className="provider-model-label" title={model}>{model}</span>
              </label>
            )
          })
        ) : (
          <div className="text-xs text-muted-foreground">{input.translations.settings.providerRefreshHint}</div>
        )}
      </div>
    </div>
  )
}

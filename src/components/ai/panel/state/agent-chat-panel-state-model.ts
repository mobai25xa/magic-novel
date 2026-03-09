import { useMemo } from 'react'

import { useSettingsStore } from '@/state/settings'

import { persistSelectedChatModel } from '../../agent-chat-model-persistence'

const DEFAULT_MODEL = 'gpt-4o-mini'

export function usePanelModelState() {
  const settingsStore = useSettingsStore()

  const availableModels = useMemo(
    () => (
      settingsStore.openaiEnabledModels.length > 0
        ? settingsStore.openaiEnabledModels
        : [DEFAULT_MODEL]
    ),
    [settingsStore.openaiEnabledModels],
  )

  const selectedModel = availableModels.includes(settingsStore.openaiModel)
    ? settingsStore.openaiModel
    : availableModels[0]

  const handleSelectModel = (model: string) => {
    settingsStore.setOpenAiModel(model)

    void persistSelectedChatModel(model).catch((error) => {
      console.error('Failed to persist selected model:', error)
    })
  }

  return {
    availableModels,
    selectedModel,
    aiChatViewMode: settingsStore.aiChatViewMode,
    setAiChatViewMode: settingsStore.setAiChatViewMode,
    sessionPersistenceEnabled: true,
    handleSelectModel,
  }
}

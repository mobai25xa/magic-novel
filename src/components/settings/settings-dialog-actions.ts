import { open as openDialog } from '@tauri-apps/plugin-dialog'

import { saveOpenAiProviderSettingsFeature } from '@/features/settings-management'
import { useProjectStore } from '@/state/project'
import { useSettingsStore } from '@/state/settings'

import {
  fetchEmbeddingModelsAction,
  fetchProviderModelsAction,
  syncProviderDraftFromStore,
} from './settings-provider-model-actions'
import { applyTempSettings, syncProjectsAfterDirectoryChange } from './settings-dialog-utils'
import type { TempState } from './use-settings-dialog-controller'

type SettingsStoreState = ReturnType<typeof useSettingsStore.getState>

type DialogActionInput = {
  onClose: () => void
  settingsStore: SettingsStoreState
  clearAllProjects: ReturnType<typeof useProjectStore.getState>['clearAllProjects']
  temp: TempState
}

async function handleSelectDirectoryAction(temp: TempState, title: string) {
  try {
    const selected = await openDialog({ directory: true, multiple: false, title })
    if (selected && typeof selected === 'string') {
      temp.setTempDir(selected)
    }
  } catch (error) {
    console.error('Failed to select directory:', error)
  }
}

async function saveProviderSettings(input: { temp: TempState; settingsStore: SettingsStoreState }) {
  try {
    const embeddingDetected = input.temp.tempOpenAiEnabledModels.includes(input.temp.tempOpenAiEmbeddingModel)
    const embeddingEnabled = input.temp.tempOpenAiEmbeddingEnabled && embeddingDetected

    const savedProvider = await saveOpenAiProviderSettingsFeature({
      openai_base_url: input.temp.tempOpenAiBaseUrl.trim(),
      openai_api_key: input.temp.tempOpenAiApiKey.trim(),
      openai_model: input.temp.tempOpenAiModel.trim() || 'gpt-4o-mini',
      openai_embedding_model: input.temp.tempOpenAiEmbeddingModel.trim() || input.temp.tempOpenAiModel.trim() || 'gpt-4o-mini',
      openai_embedding_base_url: input.temp.tempOpenAiEmbeddingBaseUrl.trim(),
      openai_embedding_api_key: input.temp.tempOpenAiEmbeddingApiKey.trim(),
      openai_local_embedding_base_url: input.temp.tempOpenAiLocalEmbeddingBaseUrl.trim(),
      openai_local_embedding_api_key: input.temp.tempOpenAiLocalEmbeddingApiKey.trim(),
      openai_embedding_source: input.temp.tempOpenAiEmbeddingSource,
      openai_embedding_enabled: embeddingEnabled,
      openai_embedding_detected: embeddingDetected,
      openai_embedding_detection_reason: embeddingDetected
        ? ''
        : (input.temp.tempOpenAiEmbeddingDetectionReason || 'embedding_model_unavailable'),
      openai_enabled_models: input.temp.tempOpenAiEnabledModels,
    })

    input.settingsStore.setOpenAiProviderSettings({
      baseUrl: savedProvider.openai_base_url,
      apiKey: savedProvider.openai_api_key,
      model: savedProvider.openai_model,
      embeddingModel: savedProvider.openai_embedding_model,
      embeddingBaseUrl: savedProvider.openai_embedding_base_url,
      embeddingApiKey: savedProvider.openai_embedding_api_key,
      localEmbeddingBaseUrl: savedProvider.openai_local_embedding_base_url,
      localEmbeddingApiKey: savedProvider.openai_local_embedding_api_key,
      embeddingSource: savedProvider.openai_embedding_source,
      embeddingEnabled: savedProvider.openai_embedding_enabled,
      embeddingAvailability: {
        enabled: savedProvider.openai_embedding_enabled,
        detected: savedProvider.openai_embedding_detected,
        reason: savedProvider.openai_embedding_detection_reason,
      },
      enabledModels: savedProvider.openai_enabled_models,
    })

    if (!savedProvider.openai_enabled_models.includes(savedProvider.openai_embedding_model)) {
      input.settingsStore.setOpenAiModel(savedProvider.openai_model)
      input.settingsStore.setOpenAiEmbeddingAvailability({
        enabled: false,
        detected: false,
        reason: 'embedding_model_unavailable',
      })
    }
  } catch (error) {
    console.error('Failed to save OpenAI provider settings:', error)
  }
}

async function handleSaveAction(input: DialogActionInput) {
  const { onClose, settingsStore, clearAllProjects, temp } = input
  const nextDir = temp.tempDir.trim()
  const dirChanged = Boolean(nextDir && nextDir !== settingsStore.projectsRootDir)

  if (nextDir) {
    settingsStore.setProjectsRootDir(nextDir)
  }

  settingsStore.setProviderType(temp.tempProviderType)

  await saveProviderSettings({ temp, settingsStore })

  applyTempSettings({
    tempGoal: temp.tempGoal,
    setDailyWordGoal: settingsStore.setDailyWordGoal,
    tempTheme: temp.tempTheme,
    setTheme: settingsStore.setTheme,
    tempLanguage: temp.tempLanguage,
    setLanguage: settingsStore.setLanguage,
    tempFirstLineIndent: temp.tempFirstLineIndent,
    setFirstLineIndent: settingsStore.setFirstLineIndent,
    tempEditorFontSize: temp.tempEditorFontSize,
    setEditorFontSize: settingsStore.setEditorFontSize,
    tempEditorLineHeight: temp.tempEditorLineHeight,
    setEditorLineHeight: settingsStore.setEditorLineHeight,
    tempEditorContentWidth: temp.tempEditorContentWidth,
    setEditorContentWidth: settingsStore.setEditorContentWidth,
    tempEditorFontFamily: temp.tempEditorFontFamily,
    setEditorFontFamily: settingsStore.setEditorFontFamily,
    tempEditorTextAlign: temp.tempEditorTextAlign,
    setEditorTextAlign: settingsStore.setEditorTextAlign,
    tempApprovalMode: temp.tempApprovalMode,
    setApprovalMode: settingsStore.setApprovalMode,
    tempCapabilityMode: temp.tempCapabilityMode,
    setCapabilityMode: settingsStore.setCapabilityMode,
    tempProjectGenres: temp.tempProjectGenres,
    setProjectGenres: settingsStore.setProjectGenres,
  })

  if (dirChanged) {
    await syncProjectsAfterDirectoryChange({
      nextDir,
      clearAllProjects,
    })
  }

  onClose()
}

function handleCancelAction(input: Pick<DialogActionInput, 'onClose' | 'settingsStore' | 'temp'>) {
  const { onClose, settingsStore, temp } = input

  temp.setTempDir(settingsStore.projectsRootDir || '')
  temp.setTempProviderType(settingsStore.providerType)
  syncProviderDraftFromStore({
    settings: {
      openaiBaseUrl: settingsStore.openaiBaseUrl,
      openaiApiKey: settingsStore.openaiApiKey,
      openaiModel: settingsStore.openaiModel,
      openaiEmbeddingModel: settingsStore.openaiEmbeddingModel,
      openaiEmbeddingBaseUrl: settingsStore.openaiEmbeddingBaseUrl,
      openaiEmbeddingApiKey: settingsStore.openaiEmbeddingApiKey,
      openaiLocalEmbeddingBaseUrl: settingsStore.openaiLocalEmbeddingBaseUrl,
      openaiLocalEmbeddingApiKey: settingsStore.openaiLocalEmbeddingApiKey,
      openaiEmbeddingSource: settingsStore.openaiEmbeddingSource,
      openaiEmbeddingEnabled: settingsStore.openaiEmbeddingEnabled,
      openaiEmbeddingDetected: settingsStore.openaiEmbeddingDetected,
      openaiEmbeddingDetectionReason: settingsStore.openaiEmbeddingDetectionReason,
      openaiEnabledModels: settingsStore.openaiEnabledModels,
    },
    temp,
  })

  temp.setTempGoal(settingsStore.dailyWordGoal)
  temp.setTempTheme(settingsStore.theme)
  temp.setTempLanguage(settingsStore.language)
  temp.setTempApprovalMode(settingsStore.approvalMode)
  temp.setTempCapabilityMode(settingsStore.capabilityMode)
  temp.setTempFirstLineIndent(settingsStore.firstLineIndent)
  temp.setTempEditorFontSize(settingsStore.editorFontSize)
  temp.setTempEditorLineHeight(settingsStore.editorLineHeight)
  temp.setTempEditorContentWidth(settingsStore.editorContentWidth)
  temp.setTempEditorFontFamily(settingsStore.editorFontFamily)
  temp.setTempEditorTextAlign(settingsStore.editorTextAlign)
  temp.setTempProjectGenres(settingsStore.projectGenres)
  temp.setNewGenre('')
  onClose()
}

export function useDialogActions(input: DialogActionInput) {
  const handleSelectDirectory = async (title: string) => handleSelectDirectoryAction(input.temp, title)
  const handleFetchModels = async () => fetchProviderModelsAction(input.temp)
  const handleFetchEmbeddingModels = async () => fetchEmbeddingModelsAction(input.temp)
  const handleSave = async () => handleSaveAction(input)
  const handleCancel = () => handleCancelAction(input)

  return {
    handleSelectDirectory,
    handleFetchModels,
    handleFetchEmbeddingModels,
    handleSave,
    handleCancel,
  }
}

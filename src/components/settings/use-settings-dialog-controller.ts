import { useEffect, useRef, useState, type MutableRefObject } from 'react'
import { loadOpenAiProviderSettingsFeature } from '@/features/settings-management'
import { useProjectStore } from '@/state/project'
import {
  useSettingsStore,
  type ApprovalMode,
  type CapabilityMode,
  type EditorFontPresetKey,
  type EditorTextAlign,
  type PlanningGenerationMode,
} from '@/state/settings'
import type { ProviderType } from '@/stores/settings-types'
import type { Language, ThemeMode } from '@/types/theme'

import {
  syncPlanningDraftFromStore,
  syncProviderDraftFromStore,
} from './settings-provider-model-actions'
import { useDialogActions } from './settings-dialog-actions'
import type { SettingsTabId } from './settings-dialog-types'

type SettingsStoreState = ReturnType<typeof useSettingsStore.getState>
type SetOpenAiProviderSettings = SettingsStoreState['setOpenAiProviderSettings']
type SetPlanningProviderSettings = SettingsStoreState['setPlanningProviderSettings']
export type TempState = ReturnType<typeof useTempSettingsState>

function useTempProviderState(store: SettingsStoreState) {
  const [tempProviderType, setTempProviderType] = useState<ProviderType>(store.providerType)
  const [tempOpenAiBaseUrl, setTempOpenAiBaseUrl] = useState(store.openaiBaseUrl)
  const [tempOpenAiApiKey, setTempOpenAiApiKey] = useState(store.openaiApiKey)
  const [tempOpenAiModel, setTempOpenAiModel] = useState(store.openaiModel)
  const [tempOpenAiEmbeddingModel, setTempOpenAiEmbeddingModel] = useState(store.openaiEmbeddingModel)
  const [tempOpenAiEmbeddingBaseUrl, setTempOpenAiEmbeddingBaseUrl] = useState(store.openaiEmbeddingBaseUrl)
  const [tempOpenAiEmbeddingApiKey, setTempOpenAiEmbeddingApiKey] = useState(store.openaiEmbeddingApiKey)
  const [tempOpenAiLocalEmbeddingBaseUrl, setTempOpenAiLocalEmbeddingBaseUrl] = useState(store.openaiLocalEmbeddingBaseUrl)
  const [tempOpenAiLocalEmbeddingApiKey, setTempOpenAiLocalEmbeddingApiKey] = useState(store.openaiLocalEmbeddingApiKey)
  const [tempOpenAiEmbeddingSource, setTempOpenAiEmbeddingSource] = useState(store.openaiEmbeddingSource)
  const [tempOpenAiEmbeddingEnabled, setTempOpenAiEmbeddingEnabled] = useState(store.openaiEmbeddingEnabled && store.openaiEmbeddingDetected)
  const [tempOpenAiEmbeddingDetected, setTempOpenAiEmbeddingDetected] = useState(store.openaiEmbeddingDetected)
  const [tempOpenAiEmbeddingDetectionReason, setTempOpenAiEmbeddingDetectionReason] = useState(store.openaiEmbeddingDetectionReason || '')
  const [tempOpenAiEnabledModels, setTempOpenAiEnabledModels] = useState<string[]>(store.openaiEnabledModels)
  const [tempFetchedModels, setTempFetchedModels] = useState<string[]>(store.openaiEnabledModels)
  const [tempFetchingModels, setTempFetchingModels] = useState(false)
  const [tempModelFetchError, setTempModelFetchError] = useState('')
  const [tempPlanningGenerationMode, setTempPlanningGenerationMode] = useState<PlanningGenerationMode>(store.planningGenerationMode)
  const [tempPlanningProviderType, setTempPlanningProviderType] = useState<ProviderType>(store.planningProviderType)
  const [tempPlanningBaseUrl, setTempPlanningBaseUrl] = useState(store.planningBaseUrl)
  const [tempPlanningApiKey, setTempPlanningApiKey] = useState(store.planningApiKey)
  const [tempPlanningModel, setTempPlanningModel] = useState(store.planningModel)
  const [tempPlanningEnabledModels, setTempPlanningEnabledModels] = useState<string[]>(store.planningEnabledModels)
  const [tempPlanningFetchedModels, setTempPlanningFetchedModels] = useState<string[]>(store.planningEnabledModels)
  const [tempPlanningFetchingModels, setTempPlanningFetchingModels] = useState(false)
  const [tempPlanningModelFetchError, setTempPlanningModelFetchError] = useState('')

  return {
    tempProviderType,
    setTempProviderType,
    tempOpenAiBaseUrl,
    setTempOpenAiBaseUrl,
    tempOpenAiApiKey,
    setTempOpenAiApiKey,
    tempOpenAiModel,
    setTempOpenAiModel,
    tempOpenAiEmbeddingModel,
    setTempOpenAiEmbeddingModel,
    tempOpenAiEmbeddingBaseUrl,
    setTempOpenAiEmbeddingBaseUrl,
    tempOpenAiEmbeddingApiKey,
    setTempOpenAiEmbeddingApiKey,
    tempOpenAiLocalEmbeddingBaseUrl,
    setTempOpenAiLocalEmbeddingBaseUrl,
    tempOpenAiLocalEmbeddingApiKey,
    setTempOpenAiLocalEmbeddingApiKey,
    tempOpenAiEmbeddingSource,
    setTempOpenAiEmbeddingSource,
    tempOpenAiEmbeddingEnabled,
    setTempOpenAiEmbeddingEnabled,
    tempOpenAiEmbeddingDetected,
    setTempOpenAiEmbeddingDetected,
    tempOpenAiEmbeddingDetectionReason,
    setTempOpenAiEmbeddingDetectionReason,
    tempOpenAiEnabledModels,
    setTempOpenAiEnabledModels,
    tempFetchedModels,
    setTempFetchedModels,
    tempFetchingModels,
    setTempFetchingModels,
    tempModelFetchError,
    setTempModelFetchError,
    tempPlanningGenerationMode,
    setTempPlanningGenerationMode,
    tempPlanningProviderType,
    setTempPlanningProviderType,
    tempPlanningBaseUrl,
    setTempPlanningBaseUrl,
    tempPlanningApiKey,
    setTempPlanningApiKey,
    tempPlanningModel,
    setTempPlanningModel,
    tempPlanningEnabledModels,
    setTempPlanningEnabledModels,
    tempPlanningFetchedModels,
    setTempPlanningFetchedModels,
    tempPlanningFetchingModels,
    setTempPlanningFetchingModels,
    tempPlanningModelFetchError,
    setTempPlanningModelFetchError,
  }
}

function useTempUiState(store: SettingsStoreState) {
  const [tempDir, setTempDir] = useState(store.projectsRootDir || '')
  const [tempGoal, setTempGoal] = useState(store.dailyWordGoal)
  const [tempTheme, setTempTheme] = useState<ThemeMode>(store.theme)
  const [tempLanguage, setTempLanguage] = useState<Language>(store.language)
  const [tempFirstLineIndent, setTempFirstLineIndent] = useState(store.firstLineIndent)
  const [tempEditorFontSize, setTempEditorFontSize] = useState(store.editorFontSize)
  const [tempEditorLineHeight, setTempEditorLineHeight] = useState(store.editorLineHeight)
  const [tempEditorContentWidth, setTempEditorContentWidth] = useState(store.editorContentWidth)
  const [tempEditorFontFamily, setTempEditorFontFamily] = useState<EditorFontPresetKey>(store.editorFontFamily)
  const [tempEditorTextAlign, setTempEditorTextAlign] = useState<EditorTextAlign>(store.editorTextAlign)
  const [tempProjectGenres, setTempProjectGenres] = useState<string[]>(store.projectGenres)
  const [tempApprovalMode, setTempApprovalMode] = useState<ApprovalMode>(store.approvalMode)
  const [tempCapabilityMode, setTempCapabilityMode] = useState<CapabilityMode>(store.capabilityMode)
  const [newGenre, setNewGenre] = useState('')

  return {
    tempDir,
    setTempDir,
    tempGoal,
    setTempGoal,
    tempTheme,
    setTempTheme,
    tempLanguage,
    setTempLanguage,
    tempFirstLineIndent,
    setTempFirstLineIndent,
    tempEditorFontSize,
    setTempEditorFontSize,
    tempEditorLineHeight,
    setTempEditorLineHeight,
    tempEditorContentWidth,
    setTempEditorContentWidth,
    tempEditorFontFamily,
    setTempEditorFontFamily,
    tempEditorTextAlign,
    setTempEditorTextAlign,
    tempProjectGenres,
    setTempProjectGenres,
    tempApprovalMode,
    setTempApprovalMode,
    tempCapabilityMode,
    setTempCapabilityMode,
    newGenre,
    setNewGenre,
  }
}

function useTempSettingsState(store: SettingsStoreState) {
  const provider = useTempProviderState(store)
  const ui = useTempUiState(store)

  return {
    ...provider,
    ...ui,
  }
}

function useProviderSyncEffect(
  open: boolean,
  setOpenAiProviderSettings: SetOpenAiProviderSettings,
  setPlanningProviderSettings: SetPlanningProviderSettings,
  setProviderType: SettingsStoreState['setProviderType'],
  temp: TempState,
  hasLoadedRef: MutableRefObject<boolean>,
) {
  useEffect(() => {
    if (!open || hasLoadedRef.current) return

    hasLoadedRef.current = true

    void (async () => {
      try {
        const provider = await loadOpenAiProviderSettingsFeature()
        setProviderType(provider.provider_type)
        setOpenAiProviderSettings({
          baseUrl: provider.openai_base_url,
          apiKey: provider.openai_api_key,
          model: provider.openai_model,
          embeddingModel: provider.openai_embedding_model,
          embeddingBaseUrl: provider.openai_embedding_base_url,
          embeddingApiKey: provider.openai_embedding_api_key,
          localEmbeddingBaseUrl: provider.openai_local_embedding_base_url,
          localEmbeddingApiKey: provider.openai_local_embedding_api_key,
          embeddingSource: provider.openai_embedding_source,
          embeddingEnabled: provider.openai_embedding_enabled,
          embeddingAvailability: {
            enabled: provider.openai_embedding_enabled,
            detected: provider.openai_embedding_detected,
            reason: provider.openai_embedding_detection_reason,
          },
          enabledModels: provider.openai_enabled_models,
        })
        setPlanningProviderSettings({
          generationMode: provider.planning_generation_mode,
          providerType: provider.planning_provider_type,
          baseUrl: provider.planning_base_url,
          apiKey: provider.planning_api_key,
          model: provider.planning_model,
          enabledModels: provider.planning_enabled_models,
        })
        syncProviderDraftFromStore({
          settings: {
            openaiBaseUrl: provider.openai_base_url,
            openaiApiKey: provider.openai_api_key,
            openaiModel: provider.openai_model,
            openaiEmbeddingModel: provider.openai_embedding_model,
            openaiEmbeddingBaseUrl: provider.openai_embedding_base_url,
            openaiEmbeddingApiKey: provider.openai_embedding_api_key,
            openaiLocalEmbeddingBaseUrl: provider.openai_local_embedding_base_url,
            openaiLocalEmbeddingApiKey: provider.openai_local_embedding_api_key,
            openaiEmbeddingSource: provider.openai_embedding_source,
            openaiEmbeddingEnabled: provider.openai_embedding_enabled,
            openaiEmbeddingDetected: provider.openai_embedding_detected,
            openaiEmbeddingDetectionReason: provider.openai_embedding_detection_reason,
            openaiEnabledModels: provider.openai_enabled_models,
          },
          temp,
        })
        syncPlanningDraftFromStore({
          settings: {
            planningGenerationMode: provider.planning_generation_mode,
            planningProviderType: provider.planning_provider_type,
            planningBaseUrl: provider.planning_base_url,
            planningApiKey: provider.planning_api_key,
            planningModel: provider.planning_model,
            planningEnabledModels: provider.planning_enabled_models,
          },
          temp,
        })
      } catch (error) {
        console.error('Failed to load OpenAI provider settings:', error)
        hasLoadedRef.current = false
      }
    })()
  }, [open, hasLoadedRef, setOpenAiProviderSettings, setPlanningProviderSettings, setProviderType, temp])
}


export function useSettingsDialogController(input: { open: boolean; onClose: () => void }) {
  const settingsStore = useSettingsStore()
  const { clearAllProjects } = useProjectStore()

  const [activeTab, setActiveTab] = useState<SettingsTabId>('about')
  const temp = useTempSettingsState(settingsStore)
  const hasLoadedRef = useRef(false)

  useEffect(() => {
    if (input.open) return
    hasLoadedRef.current = false
  }, [input.open])

  useProviderSyncEffect(
    input.open,
    settingsStore.setOpenAiProviderSettings,
    settingsStore.setPlanningProviderSettings,
    settingsStore.setProviderType,
    temp,
    hasLoadedRef,
  )

  const actions = useDialogActions({
    onClose: input.onClose,
    settingsStore,
    clearAllProjects,
    temp,
  })

  return {
    activeTab,
    setActiveTab,
    temp,
    storeLanguage: settingsStore.language,
    handleSelectDirectory: actions.handleSelectDirectory,
    handleFetchModels: actions.handleFetchModels,
    handleFetchEmbeddingModels: actions.handleFetchEmbeddingModels,
    handleFetchPlanningModels: actions.handleFetchPlanningModels,
    handleSave: actions.handleSave,
    handleCancel: actions.handleCancel,
    resetProjectGenres: settingsStore.resetProjectGenres,
  }
}

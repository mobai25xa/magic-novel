import { create } from 'zustand'
import { persist } from 'zustand/middleware'
import type { CustomThemeColors } from '@/types/theme'
import { applyThemeColors, removeThemeColors } from '@/lib/theme-utils'

import { DEFAULT_PROJECT_GENRES } from './settings-constants'
import type {
  PlanningGenerationMode,
  PlanningProviderSettingsInput,
  ProviderType,
  SettingsState,
} from './settings-types'

const DEFAULT_OPENAI_MODEL = 'gpt-4o-mini'
const DEFAULT_LOCAL_EMBEDDING_BASE_URL = 'http://127.0.0.1:11434/v1'
const DEFAULT_EMBEDDING_DETECTION_REASON = 'embedding_model_unavailable'
const DEFAULT_PROVIDER_TYPE: ProviderType = 'openai-compatible'
const DEFAULT_PLANNING_GENERATION_MODE: PlanningGenerationMode = 'follow_primary'

function normalizeGenres(input: string[]) {
  const seen = new Set<string>()
  return input
    .map((raw) => String(raw).trim())
    .filter((value) => {
      if (!value || seen.has(value)) return false
      seen.add(value)
      return true
    })
}

function normalizeModels(input: string[]) {
  const seen = new Set<string>()
  return (input || [])
    .map((item) => item.trim())
    .filter((item) => {
      if (!item || seen.has(item)) return false
      seen.add(item)
      return true
    })
}

function normalizeProviderType(input: unknown): ProviderType {
  return input === 'openai' || input === 'anthropic' || input === 'gemini'
    ? input
    : DEFAULT_PROVIDER_TYPE
}

function normalizePlanningGenerationMode(input: unknown): PlanningGenerationMode {
  return input === 'dedicated' ? 'dedicated' : DEFAULT_PLANNING_GENERATION_MODE
}

function resolvePlanningSettings(input: PlanningProviderSettingsInput) {
  const enabledModels = normalizeModels(input.enabledModels)
  const requestedModel = input.model.trim()
  const selectedModel = requestedModel || enabledModels[0] || ''
  const nextEnabledModels = enabledModels.length > 0
    ? enabledModels
    : (selectedModel ? [selectedModel] : [])

  return {
    planningGenerationMode: normalizePlanningGenerationMode(input.generationMode),
    planningProviderType: normalizeProviderType(input.providerType),
    planningBaseUrl: input.baseUrl.trim(),
    planningApiKey: input.apiKey.trim(),
    planningModel: selectedModel,
    planningEnabledModels: nextEnabledModels,
  }
}

const defaultCustomColors: CustomThemeColors = {
  light: {
    background: null,
    foreground: null,
    card: null,
    cardForeground: null,
    primary: null,
    primaryForeground: null,
    secondary: null,
    secondaryForeground: null,
    muted: null,
    mutedForeground: null,
    accent: null,
    accentForeground: null,
    border: null,
    input: null,
    ring: null,
  },
  dark: {
    background: null,
    foreground: null,
    card: null,
    cardForeground: null,
    primary: null,
    primaryForeground: null,
    secondary: null,
    secondaryForeground: null,
    muted: null,
    mutedForeground: null,
    accent: null,
    accentForeground: null,
    border: null,
    input: null,
    ring: null,
  },
}

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      projectsRootDir: null,
      providerType: 'openai-compatible' as const,
      openaiBaseUrl: '',
      openaiApiKey: '',
      openaiModel: 'gpt-4o-mini',
      openaiEmbeddingModel: 'gpt-4o-mini',
      openaiEmbeddingBaseUrl: '',
      openaiEmbeddingApiKey: '',
      openaiLocalEmbeddingBaseUrl: 'http://127.0.0.1:11434/v1',
      openaiLocalEmbeddingApiKey: '',
      openaiEmbeddingSource: 'provider',
      openaiEmbeddingEnabled: false,
      openaiEmbeddingDetected: false,
      openaiEmbeddingDetectionReason: 'embedding_model_unavailable',
      openaiEnabledModels: ['gpt-4o-mini'],
      planningGenerationMode: 'follow_primary',
      planningProviderType: 'openai-compatible',
      planningBaseUrl: '',
      planningApiKey: '',
      planningModel: '',
      planningEnabledModels: [],
      dailyWordGoal: 2000,
      theme: 'system',
      language: 'zh',
      customThemeColors: defaultCustomColors,
      firstLineIndent: true,
      editorFontSize: 16,
      editorLineHeight: 1.8,
      editorContentWidth: 750,
      editorFontFamily: 'sans',
      editorTextAlign: 'center',
      aiChatViewMode: 'compact',
      approvalMode: 'confirm_writes',
      capabilityMode: 'writing',

      projectGenres: [...DEFAULT_PROJECT_GENRES],

      setProjectsRootDir: (dir) => set({ projectsRootDir: dir }),
      setProviderType: (type) => set({ providerType: type }),
      setOpenAiProviderSettings: (input) => {
        set((state) => {
          const enabledModels = normalizeModels(input.enabledModels)
          const fallbackEnabled = enabledModels.length > 0 ? enabledModels : [input.model || DEFAULT_OPENAI_MODEL]
          const selectedModel = fallbackEnabled.includes(input.model) ? input.model : fallbackEnabled[0]
          const selectedEmbeddingModel = input.embeddingModel?.trim() || selectedModel
          const selectedEmbeddingSource = input.embeddingSource === 'local' ? 'local' : 'provider'

          const detectedFromInput = input.embeddingAvailability?.detected ?? fallbackEnabled.includes(selectedEmbeddingModel)
          const reasonFromInput = input.embeddingAvailability?.reason
            ?? (detectedFromInput ? '' : (state.openaiEmbeddingDetectionReason || DEFAULT_EMBEDDING_DETECTION_REASON))

          const requestedEnabled = input.embeddingAvailability
            ? Boolean(input.embeddingAvailability.enabled)
            : (typeof input.embeddingEnabled === 'boolean' ? input.embeddingEnabled : state.openaiEmbeddingEnabled)

          return {
            openaiBaseUrl: input.baseUrl,
            openaiApiKey: input.apiKey,
            openaiModel: selectedModel,
            openaiEmbeddingModel: selectedEmbeddingModel,
            openaiEmbeddingBaseUrl: input.embeddingBaseUrl?.trim() || input.baseUrl,
            openaiEmbeddingApiKey: input.embeddingApiKey?.trim() || input.apiKey,
            openaiLocalEmbeddingBaseUrl: input.localEmbeddingBaseUrl?.trim() || DEFAULT_LOCAL_EMBEDDING_BASE_URL,
            openaiLocalEmbeddingApiKey: input.localEmbeddingApiKey?.trim() || '',
            openaiEmbeddingSource: selectedEmbeddingSource,
            openaiEmbeddingEnabled: requestedEnabled && detectedFromInput,
            openaiEmbeddingDetected: detectedFromInput,
            openaiEmbeddingDetectionReason: reasonFromInput,
            openaiEnabledModels: fallbackEnabled,
          }
        })
      },
      setOpenAiModel: (model) =>
        set((state) => {
          const next = model.trim()
          if (!next) return {}
          if (!state.openaiEnabledModels.includes(next)) return {}
          return { openaiModel: next }
        }),
      setPlanningGenerationMode: (mode) =>
        set({ planningGenerationMode: normalizePlanningGenerationMode(mode) }),
      setPlanningProviderSettings: (input) => set(resolvePlanningSettings(input)),
      setOpenAiEmbeddingSource: (source) =>
        set({ openaiEmbeddingSource: source === 'local' ? 'local' : 'provider' }),
      setOpenAiEmbeddingEnabled: (enabled) =>
        set((state) => ({
          openaiEmbeddingEnabled: state.openaiEmbeddingDetected ? enabled : false,
        })),
      setOpenAiEmbeddingAvailability: (input) =>
        set(() => ({
          openaiEmbeddingDetected: Boolean(input.detected),
          openaiEmbeddingDetectionReason: input.reason,
          openaiEmbeddingEnabled: Boolean(input.enabled) && Boolean(input.detected),
        })),
      setDailyWordGoal: (goal) => set({ dailyWordGoal: goal }),
      setTheme: (theme) => set({ theme }),
      setLanguage: (language) => set({ language }),
      setCustomThemeColors: (colors) => {
        set({ customThemeColors: colors })
        applyThemeColors(colors)
      },
      resetCustomThemeColors: () => {
        set({ customThemeColors: defaultCustomColors })
        removeThemeColors()
      },
      setFirstLineIndent: (enabled) => set({ firstLineIndent: enabled }),
      setEditorFontSize: (size) => set({ editorFontSize: size }),
      setEditorLineHeight: (height) => set({ editorLineHeight: height }),
      setEditorContentWidth: (width) => set({ editorContentWidth: width }),
      setEditorFontFamily: (family) => set({ editorFontFamily: family }),
      setEditorTextAlign: (align) => set({ editorTextAlign: align }),
      setAiChatViewMode: (mode) => set({ aiChatViewMode: mode }),
      setApprovalMode: (mode) => set({ approvalMode: mode }),
      setCapabilityMode: (mode) => set({ capabilityMode: mode }),

      setProjectGenres: (genres) => set({ projectGenres: normalizeGenres(genres) }),
      addProjectGenre: (genre) =>
        set((state) => ({
          projectGenres: normalizeGenres([...state.projectGenres, genre]),
        })),
      removeProjectGenre: (genre) =>
        set((state) => ({
          projectGenres: state.projectGenres.filter((g) => g !== genre),
        })),
      resetProjectGenres: () => set({ projectGenres: [...DEFAULT_PROJECT_GENRES] }),
    }),
    {
      name: 'magic-novel-settings',
      version: 4,
      migrate: (persistedState: unknown) => {
        if (!persistedState || typeof persistedState !== 'object') {
          return persistedState as SettingsState
        }

        const next = { ...(persistedState as Record<string, unknown>) }
        delete next.aiChatUiV2
        delete next.agentSessionPersistenceEnabled
        delete next.agentEngineV2Enabled
        delete next.magicAgentTestMode
        delete next.agentSessionCompactionV2Enabled
        delete next.agentSessionReminderEnabled

        if (next.approvalMode !== 'confirm_writes' && next.approvalMode !== 'auto') {
          next.approvalMode = 'confirm_writes'
        }

        if (next.capabilityMode !== 'writing' && next.capabilityMode !== 'planning') {
          next.capabilityMode = 'writing'
        }

        next.providerType = normalizeProviderType(next.providerType)
        Object.assign(next, resolvePlanningSettings({
          generationMode: normalizePlanningGenerationMode(next.planningGenerationMode),
          providerType: normalizeProviderType(next.planningProviderType),
          baseUrl: typeof next.planningBaseUrl === 'string' ? next.planningBaseUrl : '',
          apiKey: typeof next.planningApiKey === 'string' ? next.planningApiKey : '',
          model: typeof next.planningModel === 'string' ? next.planningModel : '',
          enabledModels: Array.isArray(next.planningEnabledModels)
            ? next.planningEnabledModels.filter((item): item is string => typeof item === 'string')
            : [],
        }))

        return next as unknown as SettingsState
      },
    }
  )
)

import type { ThemeMode, Language, CustomThemeColors } from '@/types/theme'

import type { EditorFontPresetKey, EditorTextAlign } from './settings-constants'

export type AiChatViewMode = 'compact' | 'debug'
export type EmbeddingSource = 'provider' | 'local'
export type ProviderType = 'openai' | 'anthropic' | 'gemini' | 'openai-compatible'
export type ApprovalMode = 'confirm_writes' | 'auto'
export type CapabilityMode = 'writing' | 'planning'

export interface EmbeddingAvailability {
  enabled: boolean
  detected: boolean
  reason?: string
}

export interface OpenAiProviderSettingsInput {
  baseUrl: string
  apiKey: string
  model: string
  embeddingModel?: string
  embeddingBaseUrl?: string
  embeddingApiKey?: string
  localEmbeddingBaseUrl?: string
  localEmbeddingApiKey?: string
  embeddingSource?: EmbeddingSource
  embeddingEnabled?: boolean
  embeddingAvailability?: EmbeddingAvailability
  enabledModels: string[]
}

export interface SettingsState {
  projectsRootDir: string | null

  providerType: ProviderType
  openaiBaseUrl: string
  openaiApiKey: string
  openaiModel: string
  openaiEmbeddingModel: string
  openaiEmbeddingBaseUrl: string
  openaiEmbeddingApiKey: string
  openaiLocalEmbeddingBaseUrl: string
  openaiLocalEmbeddingApiKey: string
  openaiEmbeddingSource: EmbeddingSource
  openaiEmbeddingEnabled: boolean
  openaiEmbeddingDetected: boolean
  openaiEmbeddingDetectionReason?: string
  openaiEnabledModels: string[]

  dailyWordGoal: number
  theme: ThemeMode
  language: Language
  customThemeColors: CustomThemeColors

  firstLineIndent: boolean
  editorFontSize: number
  editorLineHeight: number
  editorContentWidth: number
  editorFontFamily: EditorFontPresetKey
  editorTextAlign: EditorTextAlign

  aiChatViewMode: AiChatViewMode
  approvalMode: ApprovalMode
  capabilityMode: CapabilityMode

  projectGenres: string[]

  setProjectsRootDir: (dir: string | null) => void
  setProviderType: (type: ProviderType) => void
  setOpenAiProviderSettings: (input: OpenAiProviderSettingsInput) => void
  setOpenAiModel: (model: string) => void
  setOpenAiEmbeddingSource: (source: EmbeddingSource) => void
  setOpenAiEmbeddingEnabled: (enabled: boolean) => void
  setOpenAiEmbeddingAvailability: (input: EmbeddingAvailability) => void
  setDailyWordGoal: (goal: number) => void
  setTheme: (theme: ThemeMode) => void
  setLanguage: (language: Language) => void
  setCustomThemeColors: (colors: CustomThemeColors) => void
  resetCustomThemeColors: () => void
  setFirstLineIndent: (enabled: boolean) => void
  setEditorFontSize: (size: number) => void
  setEditorLineHeight: (height: number) => void
  setEditorContentWidth: (width: number) => void
  setEditorFontFamily: (family: EditorFontPresetKey) => void
  setEditorTextAlign: (align: EditorTextAlign) => void
  setAiChatViewMode: (mode: AiChatViewMode) => void
  setApprovalMode: (mode: ApprovalMode) => void
  setCapabilityMode: (mode: CapabilityMode) => void
  setProjectGenres: (genres: string[]) => void
  addProjectGenre: (genre: string) => void
  removeProjectGenre: (genre: string) => void
  resetProjectGenres: () => void
}

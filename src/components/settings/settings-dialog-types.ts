import type {
  ApprovalMode,
  CapabilityMode,
  EditorFontPresetKey,
  EditorTextAlign,
  EmbeddingSource,
} from '@/state/settings'
import type { ProviderType } from '@/stores/settings-types'
import type { Language, ThemeMode } from '@/types/theme'

export type SettingsTabId = 'about' | 'general' | 'providers' | 'editor' | 'projects' | 'ai'

export type TempSettingsState = {
  tempDir: string
  tempProviderType: ProviderType
  tempOpenAiBaseUrl: string
  tempOpenAiApiKey: string
  tempOpenAiModel: string
  tempOpenAiEmbeddingModel: string
  tempOpenAiEmbeddingBaseUrl: string
  tempOpenAiEmbeddingApiKey: string
  tempOpenAiLocalEmbeddingBaseUrl: string
  tempOpenAiLocalEmbeddingApiKey: string
  tempOpenAiEmbeddingSource: EmbeddingSource
  tempOpenAiEmbeddingEnabled: boolean
  tempOpenAiEmbeddingDetected: boolean
  tempOpenAiEmbeddingDetectionReason: string
  tempOpenAiEnabledModels: string[]
  tempFetchedModels: string[]
  tempFetchingModels: boolean
  tempModelFetchError: string
  tempGoal: number
  tempTheme: ThemeMode
  tempLanguage: Language
  tempFirstLineIndent: boolean
  tempEditorFontSize: number
  tempEditorLineHeight: number
  tempEditorContentWidth: number
  tempEditorFontFamily: EditorFontPresetKey
  tempEditorTextAlign: EditorTextAlign
  tempApprovalMode: ApprovalMode
  tempCapabilityMode: CapabilityMode
  tempProjectGenres: string[]
  newGenre: string
}

export type TempSettingsSetters = {
  setTempDir: (value: string) => void
  setTempProviderType: (value: ProviderType) => void
  setTempOpenAiBaseUrl: (value: string) => void
  setTempOpenAiApiKey: (value: string) => void
  setTempOpenAiModel: (value: string) => void
  setTempOpenAiEmbeddingModel: (value: string) => void
  setTempOpenAiEmbeddingBaseUrl: (value: string) => void
  setTempOpenAiEmbeddingApiKey: (value: string) => void
  setTempOpenAiLocalEmbeddingBaseUrl: (value: string) => void
  setTempOpenAiLocalEmbeddingApiKey: (value: string) => void
  setTempOpenAiEmbeddingSource: (value: EmbeddingSource) => void
  setTempOpenAiEmbeddingEnabled: React.Dispatch<React.SetStateAction<boolean>>
  setTempOpenAiEmbeddingDetected: (value: boolean) => void
  setTempOpenAiEmbeddingDetectionReason: (value: string) => void
  setTempOpenAiEnabledModels: React.Dispatch<React.SetStateAction<string[]>>
  setTempFetchedModels: (value: string[]) => void
  setTempFetchingModels: (value: boolean) => void
  setTempModelFetchError: (value: string) => void
  setTempGoal: (value: number) => void
  setTempTheme: (value: ThemeMode) => void
  setTempLanguage: (value: Language) => void
  setTempFirstLineIndent: (value: boolean) => void
  setTempEditorFontSize: (value: number) => void
  setTempEditorLineHeight: (value: number) => void
  setTempEditorContentWidth: (value: number) => void
  setTempEditorFontFamily: (value: EditorFontPresetKey) => void
  setTempEditorTextAlign: (value: EditorTextAlign) => void
  setTempApprovalMode: (value: ApprovalMode) => void
  setTempCapabilityMode: (value: CapabilityMode) => void
  setTempProjectGenres: React.Dispatch<React.SetStateAction<string[]>>
  setNewGenre: (value: string) => void
}

export type SettingsDialogTranslations = {
  settings: Record<string, string> & {
    providerEmbeddingModel?: string
    providerEmbeddingType?: string
    providerEmbeddingDescription?: string
    providerEmbeddingSource?: string
    providerEmbeddingSourceProvider?: string
    providerEmbeddingSourceLocal?: string
    providerEmbeddingEnabled?: string
    providerEmbeddingUnavailable?: string
    providerEmbeddingBaseUrl?: string
    providerEmbeddingApiKey?: string
    providerLocalEmbeddingBaseUrl?: string
    providerLocalEmbeddingApiKey?: string
    providerEmbeddingForChat?: string
    providerEmbeddingForSearch?: string
    providerEmbeddingForChatAndSearch?: string
  }
  common: Record<string, string>
  home: Record<string, string>
  projectType: Record<string, string>
  settingsExtra: Record<string, string>
  ai: {
    panel: Record<string, string>
  }
}

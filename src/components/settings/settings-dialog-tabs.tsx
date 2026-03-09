import { Bot, Brain, FolderCog, Info, Settings, Type } from 'lucide-react'
import type { ReactNode } from 'react'

import type { SettingsDialogTranslations, SettingsTabId } from './settings-dialog-types'

type SettingsTabItem = {
  id: SettingsTabId
  label: string
  icon: ReactNode
}

export function buildSettingsTabs(translations: SettingsDialogTranslations): SettingsTabItem[] {
  return [
    { id: 'about', label: translations.settings.about, icon: <Info className="h-4 w-4" /> },
    { id: 'general', label: translations.settings.general, icon: <Settings className="h-4 w-4" /> },
    { id: 'providers', label: translations.settings.providers, icon: <Brain className="h-4 w-4" /> },
    { id: 'editor', label: translations.settings.editor, icon: <Type className="h-4 w-4" /> },
    { id: 'projects', label: translations.settings.projects, icon: <FolderCog className="h-4 w-4" /> },
    { id: 'ai', label: translations.settings.aiSettings, icon: <Bot className="h-4 w-4" /> },
  ]
}

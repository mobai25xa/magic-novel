import { Bot, Brain, FolderCog, Info, Settings, Type } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'

import { renderSettingsDialogContent } from './settings-dialog-content'
import { SettingsDialogLayout } from './settings-dialog-layout'
import type { SettingsDialogTranslations } from './settings-dialog-types'
import { buildSettingsTabs } from './settings-dialog-tabs'
import { useSettingsDialogController } from './use-settings-dialog-controller'

interface SettingsDialogProps {
  open: boolean
  onClose: () => void
}

function buildFallbackTabs(translations: SettingsDialogTranslations) {
  return [
    { id: 'about' as const, label: translations.settings.about, icon: <Info className="h-4 w-4" /> },
    { id: 'general' as const, label: translations.settings.general, icon: <Settings className="h-4 w-4" /> },
    { id: 'providers' as const, label: translations.settings.providers, icon: <Brain className="h-4 w-4" /> },
    { id: 'editor' as const, label: translations.settings.editor, icon: <Type className="h-4 w-4" /> },
    { id: 'projects' as const, label: translations.settings.projects, icon: <FolderCog className="h-4 w-4" /> },
    { id: 'ai' as const, label: translations.settings.aiSettings, icon: <Bot className="h-4 w-4" /> },
  ]
}

export function SettingsDialog({ open, onClose }: SettingsDialogProps) {
  const { translations } = useTranslation()
  const controller = useSettingsDialogController({ open, onClose })

  const typedTranslations = translations as unknown as SettingsDialogTranslations
  const tabs = buildSettingsTabs(typedTranslations)
  const safeTabs = tabs.length > 0 ? tabs : buildFallbackTabs(typedTranslations)

  const content = renderSettingsDialogContent({
    activeTab: controller.activeTab,
    translations: typedTranslations,
    language: controller.storeLanguage,
    temp: controller.temp,
    onSelectDirectory: () => controller.handleSelectDirectory(translations.settings.selectRootDirectory),
    onFetchModels: controller.handleFetchModels,
    onFetchEmbeddingModels: controller.handleFetchEmbeddingModels,
    onFetchPlanningModels: controller.handleFetchPlanningModels,
    resetProjectGenres: controller.resetProjectGenres,
  })

  return (
    <SettingsDialogLayout
      open={open}
      title={translations.settings.title}
      tabs={safeTabs}
      activeTab={controller.activeTab}
      setActiveTab={controller.setActiveTab}
      content={content}
      cancelLabel={translations.common.cancel}
      saveLabel={translations.common.save}
      onCancel={controller.handleCancel}
      onSave={controller.handleSave}
    />
  )
}

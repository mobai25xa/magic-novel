import { useTranslation } from '@/hooks/use-translation'
import { useNavigationStore } from '@/stores/navigation-store'
import { renderSettingsDialogContent } from './settings-dialog-content'
import { SettingsPageLayout } from './SettingsPageLayout'
import type { SettingsDialogTranslations } from './settings-dialog-types'
import type { SettingsSubPage } from '@/types/navigation'
import { useSettingsDialogController } from './use-settings-dialog-controller'

/**
 * 全页面设置组件 — 通过 GlobalSidebar 导航进入
 * 复用已有的 settings-dialog-content 渲染逻辑
 */
export function SettingsPage() {
  const { translations } = useTranslation()
  const settingsSubPage = useNavigationStore((s) => s.settingsSubPage)
  const setSettingsSubPage = useNavigationStore((s) => s.setSettingsSubPage)
  const goBack = useNavigationStore((s) => s.goBack)

  // 复用已有 controller，open 始终为 true
  const controller = useSettingsDialogController({
    open: true,
    onClose: goBack,
  })

  const typedTranslations = translations as unknown as SettingsDialogTranslations

  // 同步 navigation-store 的 subPage 到 controller
  const activeTab = settingsSubPage as typeof controller.activeTab

  const content = renderSettingsDialogContent({
    activeTab,
    translations: typedTranslations,
    language: controller.storeLanguage,
    temp: controller.temp,
    onSelectDirectory: () => controller.handleSelectDirectory(translations.settings.selectRootDirectory),
    onFetchModels: controller.handleFetchModels,
    onFetchEmbeddingModels: controller.handleFetchEmbeddingModels,
    onFetchPlanningModels: controller.handleFetchPlanningModels,
    resetProjectGenres: controller.resetProjectGenres,
  })

  const handleTabChange = (tab: SettingsSubPage) => {
    setSettingsSubPage(tab)
    controller.setActiveTab(tab as typeof controller.activeTab)
  }

  return (
    <SettingsPageLayout
      activeTab={activeTab as SettingsSubPage}
      onTabChange={handleTabChange}
      onBack={goBack}
      onSave={controller.handleSave}
      onCancel={() => {
        controller.handleCancel()
        goBack()
      }}
    >
      {content}
    </SettingsPageLayout>
  )
}

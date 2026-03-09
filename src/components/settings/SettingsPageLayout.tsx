import type { ReactNode } from 'react'
import { useT } from '@/hooks/use-translation'
import { SettingsSidebar } from './SettingsSidebar'
import { SettingsButton } from './settings-dialog-button'
import type { SettingsSubPage } from '@/types/navigation'

interface SettingsPageLayoutProps {
  activeTab: SettingsSubPage
  onTabChange: (tab: SettingsSubPage) => void
  onBack: () => void
  onSave: () => void
  onCancel: () => void
  children: ReactNode
}

const TAB_TITLES: Record<SettingsSubPage, { titleKey: string; subtitleKey?: string }> = {
  about: { titleKey: 'settings.about' },
  general: { titleKey: 'settings.general' },
  providers: { titleKey: 'settings.providers', subtitleKey: 'settings.providersDescription' },
  editor: { titleKey: 'settings.editor' },
  projects: { titleKey: 'settings.projects' },
  ai: { titleKey: 'settings.aiSettings', subtitleKey: 'settings.aiSettingsDescription' },
}

export function SettingsPageLayout({
  activeTab,
  onTabChange,
  onBack,
  onSave,
  onCancel,
  children,
}: SettingsPageLayoutProps) {
  const t = useT()
  const tabMeta = TAB_TITLES[activeTab]

  return (
    <div className="settings-page-shell">
      <SettingsSidebar activeTab={activeTab} onTabChange={onTabChange} onBack={onBack} />

      <div className="settings-content-area" style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '24px 24px 24px 0' }}>
        <div className="glass-panel-subtle" style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden', borderRadius: 24 }}>
          {/* 头部 */}
          <div className="content-header">
            <div>
              <h1 className="page-title">{t(tabMeta.titleKey)}</h1>
              {tabMeta.subtitleKey && (
                <p className="page-subtitle">{t(tabMeta.subtitleKey)}</p>
              )}
            </div>
          </div>

          {/* 内容滚动区 */}
          <div className="content-scroll" style={{ flex: 1, overflowY: 'auto', padding: '24px 36px' }}>
            {children}
          </div>

          {/* 底部操作 */}
          <div className="content-footer">
            <SettingsButton variant="outline" onClick={onCancel}>{t('common.cancel')}</SettingsButton>
            <SettingsButton onClick={onSave}>{t('common.save')}</SettingsButton>
          </div>
        </div>
      </div>
    </div>
  )
}

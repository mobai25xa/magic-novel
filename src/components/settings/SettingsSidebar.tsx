import { ArrowLeft, Info, Settings, Brain, Type, FolderCog, Bot } from 'lucide-react'
import { useT } from '@/hooks/use-translation'
import type { SettingsSubPage } from '@/types/navigation'

interface SettingsSidebarProps {
  activeTab: SettingsSubPage
  onTabChange: (tab: SettingsSubPage) => void
  onBack: () => void
}

const SETTINGS_TABS: { id: SettingsSubPage; icon: typeof Info; labelKey: string }[] = [
  { id: 'about', icon: Info, labelKey: 'settings.about' },
  { id: 'general', icon: Settings, labelKey: 'settings.general' },
  { id: 'providers', icon: Brain, labelKey: 'settings.providers' },
  { id: 'editor', icon: Type, labelKey: 'settings.editor' },
  { id: 'projects', icon: FolderCog, labelKey: 'settings.projects' },
  { id: 'ai', icon: Bot, labelKey: 'settings.aiSettings' },
]

export function SettingsSidebar({ activeTab, onTabChange, onBack }: SettingsSidebarProps) {
  const t = useT()

  return (
    <aside className="settings-sidebar">
      <div className="nav-back-section">
        <button className="nav-item back" onClick={onBack}>
          <ArrowLeft size={16} />
          <span>{t('nav.back')}</span>
        </button>
      </div>

      <nav className="nav-menu settings-nav">
        {SETTINGS_TABS.map(({ id, icon: Icon, labelKey }) => (
          <button
            key={id}
            className={`nav-item ${activeTab === id ? 'active' : ''}`}
            onClick={() => onTabChange(id)}
          >
            <Icon size={18} />
            <span>{t(labelKey)}</span>
          </button>
        ))}
      </nav>
    </aside>
  )
}

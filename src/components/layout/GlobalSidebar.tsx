import { Home, BarChart3, Sparkles, Cpu, Trash2, Settings } from 'lucide-react'
import { useT } from '@/hooks/use-translation'
import { useProjectStore } from '@/state/project'
import type { AppPage } from '@/types/navigation'

interface GlobalSidebarProps {
  currentPage: AppPage
  collapsed: boolean
  onNavigate: (page: AppPage) => void
  onToggleCollapse: () => void
  recentItems?: { title: string; path: string }[]
}

const NAV_ITEMS: { page: AppPage; icon: typeof Home }[] = [
  { page: 'workspace', icon: Home },
  { page: 'dashboard', icon: BarChart3 },
  { page: 'skills', icon: Sparkles },
  { page: 'workers', icon: Cpu },
  { page: 'recycle', icon: Trash2 },
]

const PAGE_I18N_KEY: Record<string, string> = {
  workspace: 'nav.workspace',
  dashboard: 'nav.dashboard',
  skills: 'nav.skills',
  workers: 'nav.workers',
  recycle: 'nav.recycle',
  settings: 'nav.settings',
}

export function GlobalSidebar({
  currentPage,
  collapsed,
  onNavigate,
  onToggleCollapse,
  recentItems,
}: GlobalSidebarProps) {
  const t = useT()
  const projectList = useProjectStore((s) => s.projectList)

  const recent = recentItems ?? projectList.slice(0, 5).map((p) => ({
    title: p.name,
    path: p.path,
  }))

  return (
    <aside className={`global-sidebar glass-panel ${collapsed ? 'collapsed' : ''}`}>
      <div className="sidebar-header" onDoubleClick={onToggleCollapse}>
        <div className="brand-icon">
          <img src="/icon.png" alt="Magic Novel" className="brand-icon-image" />
        </div>
        <span className="brand-text">{t('nav.brandName')}</span>
      </div>

      <nav className="nav-menu">
        <div className="nav-primary-group">
          {NAV_ITEMS.map(({ page, icon: Icon }) => (
            <button
              key={page}
              className={`nav-item ${currentPage === page ? 'active' : ''}`}
              onClick={() => onNavigate(page)}
            >
              <Icon size={20} className="nav-item-icon" />
              <span className="nav-item-text">{t(PAGE_I18N_KEY[page])}</span>
            </button>
          ))}
        </div>

        {recent.length > 0 && (
          <>
            <div className="nav-label">{t('nav.recentProjects')}</div>
            {recent.slice(0, 2).map((item, index) => (
              <button
                key={item.path}
                className="recent-item"
                onClick={() => {
                  const { setProjectPath } = useProjectStore.getState()
                  setProjectPath(item.path)
                  onNavigate('editor')
                }}
              >
                <span className={`recent-item-dot dot-${index % 2 === 0 ? 'blue' : 'green'}`} />
                <span className="recent-item-name">{item.title}</span>
              </button>
            ))}
          </>
        )}
      </nav>

      <div className="sidebar-footer">
        <button
          className={`nav-item ${currentPage === 'settings' ? 'active' : ''}`}
          onClick={() => onNavigate('settings')}
        >
          <Settings size={20} className="nav-item-icon" />
          <span className="nav-item-text">{t('nav.settings')}</span>
        </button>
      </div>
    </aside>
  )
}

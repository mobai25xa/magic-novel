import { Bot, Compass, FileText, Home, Trash } from 'lucide-react'

import type { HomeTab } from './home-page-types'

type Input = {
  activeTab: HomeTab
  homeLabel: string
  discoverLabel: string
  skillsLabel: string
  workersLabel: string
  recycleLabel: string
  setActiveTab: (tab: HomeTab) => void
}

function SidebarTab(input: {
  active: boolean
  icon: React.ReactNode
  label: string
  onClick: () => void
}) {
  return (
    <button
      onClick={input.onClick}
      className={`sidebar-tab ${input.active ? 'sidebar-tab-active' : ''}`}
    >
      {input.icon}
      {input.label}
    </button>
  )
}

export function HomePageSidebar(input: Input) {
  return (
    <aside className="sidebar">
      <nav className="p-3 space-y-1">
        <SidebarTab
          active={input.activeTab === 'home'}
          icon={<Home className="h-4 w-4" />}
          label={input.homeLabel}
          onClick={() => input.setActiveTab('home')}
        />
        <SidebarTab
          active={input.activeTab === 'discover'}
          icon={<Compass className="h-4 w-4" />}
          label={input.discoverLabel}
          onClick={() => input.setActiveTab('discover')}
        />
        <SidebarTab
          active={input.activeTab === 'skills'}
          icon={<FileText className="h-4 w-4" />}
          label={input.skillsLabel}
          onClick={() => input.setActiveTab('skills')}
        />
        <SidebarTab
          active={input.activeTab === 'workers'}
          icon={<Bot className="h-4 w-4" />}
          label={input.workersLabel}
          onClick={() => input.setActiveTab('workers')}
        />
        <SidebarTab
          active={input.activeTab === 'recycle'}
          icon={<Trash className="h-4 w-4" />}
          label={input.recycleLabel}
          onClick={() => input.setActiveTab('recycle')}
        />
      </nav>
    </aside>
  )
}

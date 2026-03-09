import { DiscoverPage } from '@/pages/discover'
import { SkillsPage } from '@/pages/skills'
import { WorkersPage } from '@/pages/workers'
import { BookOpen, PenTool } from 'lucide-react'

import { HeroCard } from '../HeroCard'
import { StatCard } from '../StatCard'
import { AiTipCard } from '../AiTipCard'
import { BookShelf } from '../BookShelf'
import { HomePageRecycleGrid } from './home-page-project-grid'
import type { HomeTab } from './home-page-types'

type Input = {
  activeTab: HomeTab
  projectList: Array<{ path: string; name: string; lastOpenedAt: number; coverImage?: string }>
  filteredProjects: Array<{ path: string; name: string; lastOpenedAt: number; coverImage?: string }>
  recycledProjects: Array<{ id: string; path: string; name: string; deletedAt: number; coverImage?: string }>
  totalWordCount: number
  todayWordDelta: number
  popularType: { type: string; count: number } | null
  typeFilter: string | null
  translations: ReturnType<typeof import('@/hooks/use-translation').useTranslation>['translations']
  onToggleTypeFilter: (type: string) => void
  onOpenProject: (path: string) => void
  onProjectContextMenu: (path: string, event: React.MouseEvent) => void
  onRestoreProject: (id: string, path: string, event: React.MouseEvent) => void
}

export function HomePageMainContent(input: Input) {
  const todayDeltaText = `${input.translations.discover.today} +${input.todayWordDelta.toLocaleString()} ${input.translations.discover.wordUnit}`

  return (
    <main className="main-content home-main-content">
      {input.activeTab === 'home' ? (
        <div className="bento-grid">
          <HeroCard onOpenProject={input.onOpenProject} />

          <StatCard
            icon={<BookOpen size={16} />}
            value={input.totalWordCount.toLocaleString()}
            label={input.translations.home.totalWords}
            trend={{ direction: 'up', value: todayDeltaText }}
            iconTone="blue"
          />

          <StatCard
            icon={<PenTool size={16} />}
            value={input.projectList.length}
            label="作品数量"
            iconTone="green"
          />

          <AiTipCard />

          <BookShelf
            onOpenProject={input.onOpenProject}
            onCreateProject={() => {
              // 触发创建项目对话框
              const event = new CustomEvent('open-create-project-dialog')
              window.dispatchEvent(event)
            }}
          />
        </div>
      ) : null}

      {input.activeTab === 'discover' ? <DiscoverPage /> : null}

      {input.activeTab === 'skills' ? <SkillsPage /> : null}

      {input.activeTab === 'workers' ? <WorkersPage /> : null}

      {input.activeTab === 'recycle' ? (
        <div className="max-w-[1600px] mx-auto px-8 py-8">
          <HomePageRecycleGrid
            recycledProjects={input.recycledProjects}
            translations={input.translations}
            onProjectContextMenu={input.onProjectContextMenu}
            onRestoreProject={input.onRestoreProject}
          />
        </div>
      ) : null}
    </main>
  )
}

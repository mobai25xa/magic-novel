import { FolderOpen, Trash2 } from 'lucide-react'

import { Badge, Tooltip, TooltipContent, TooltipTrigger } from '@/magic-ui/components'

import { formatRelativeDate } from '../home-utils'

type ProjectItem = {
  path: string
  name: string
  lastOpenedAt: number
  coverImage?: string
}

type RecycledItem = {
  id: string
  path: string
  name: string
  deletedAt: number
  coverImage?: string
}

type HomeTranslations = ReturnType<typeof import('@/hooks/use-translation').useTranslation>['translations']['home']

export function HomePageProjectGrid(input: {
  filteredProjects: ProjectItem[]
  language: string
  emptyTitle: string
  emptyDescription: string
  serializingLabel: string
  onOpenProject: (path: string) => void
  onProjectContextMenu: (path: string, event: React.MouseEvent) => void
}) {
  if (input.filteredProjects.length <= 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
        <FolderOpen className="h-16 w-16 mb-4 opacity-50" />
        <p className="text-lg mb-2">{input.emptyTitle}</p>
        <p className="text-sm">{input.emptyDescription}</p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-4 gap-5 mb-8">
      {input.filteredProjects.map((project, index) => {
        const accentColors = [
          '#10b981', '#3b82f6', '#ec4899', '#f59e0b', '#8b5cf6', '#06b6d4',
        ]
        const accentColor = accentColors[index % accentColors.length]
        return (
          <div
            key={project.path}
            className="h-card group cursor-pointer"
            onClick={() => input.onOpenProject(project.path)}
            onContextMenu={(event) => input.onProjectContextMenu(project.path, event)}
          >
            {/* 左侧封面图 */}
            <div className="h-card-cover">
              {project.coverImage ? (
                <img src={project.coverImage} alt={project.name} className="w-full h-full object-cover" />
              ) : (
                <div className="h-card-cover-placeholder" style={{ background: `linear-gradient(135deg, ${accentColor}22 0%, ${accentColor}44 100%)` }}>
                  <img src="/icon.png" alt="" className="w-10 h-10 opacity-40" />
                </div>
              )}
              {/* 左侧色条 */}
              <div className="h-card-accent" style={{ backgroundColor: accentColor }} />
            </div>

            {/* 右侧信息 */}
            <div className="h-card-body">
              <div className="h-card-top">
                <Badge color="success" className="h-card-badge">{input.serializingLabel}</Badge>
              </div>
              <h3 className="h-card-title">{project.name}</h3>
              <p className="h-card-date">
                {new Date(project.lastOpenedAt).toLocaleDateString(
                  input.language === 'en' ? 'en-US' : 'zh-CN'
                )}
              </p>
            </div>
          </div>
        )
      })}
    </div>
  )
}

export function HomePageRecycleGrid(input: {
  recycledProjects: RecycledItem[]
  translations: { home: HomeTranslations }
  onProjectContextMenu: (path: string, event: React.MouseEvent) => void
  onRestoreProject: (id: string, path: string, event: React.MouseEvent) => void
}) {
  if (input.recycledProjects.length <= 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
        <Trash2 className="h-16 w-16 mb-4 opacity-50" />
        <p className="text-lg">{input.translations.home.recycleBinEmpty}</p>
      </div>
    )
  }

  return (
    <div className="grid grid-cols-4 gap-4 mb-8">
      {input.recycledProjects.map((project) => (
        <div
          key={project.path}
          className="relative rounded-lg overflow-hidden group h-48 opacity-75 hover:opacity-100 transition-all"
          onContextMenu={(event) => input.onProjectContextMenu(project.path, event)}
        >
          {project.coverImage ? (
            <div className="absolute inset-0">
              <img src={project.coverImage} alt={project.name} className="w-full h-full object-cover grayscale" />
            </div>
          ) : (
            <div className="absolute inset-0 flex items-center justify-center project-card-bg">
              <img src="/icon.png" alt="" className="w-20 h-20 opacity-20 grayscale" />
            </div>
          )}

          <div className="absolute inset-0 bg-gradient-to-t from-black/80 via-black/40 to-transparent" />

          <div className="absolute top-3 right-3 z-20">
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  onClick={(event) => input.onRestoreProject(project.id, project.path, event)}
                  className="restore-fab opacity-0 group-hover:opacity-100 transition-opacity"
                >
                  <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                    />
                  </svg>
                </button>
              </TooltipTrigger>
              <TooltipContent variant="success">{input.translations.home.restore}</TooltipContent>
            </Tooltip>
          </div>

          <div className="absolute top-3 left-3 z-10">
            <Badge color="error">{input.translations.home.deleted}</Badge>
          </div>

          <div className="absolute bottom-0 left-0 right-0 p-4 text-white z-10">
            <h3 className="font-bold text-lg mb-1">{project.name}</h3>
            <p className="text-sm text-white/80">
              {input.translations.home.deletedAt}{' '}
              {formatRelativeDate(project.deletedAt, {
                today: input.translations.home.today,
                yesterday: input.translations.home.yesterday,
                daysAgo: input.translations.home.daysAgo,
                weeksAgo: input.translations.home.weeksAgo,
                monthsAgo: input.translations.home.monthsAgo,
                yearsAgo: input.translations.home.yearsAgo,
              })}
            </p>
          </div>
        </div>
      ))}
    </div>
  )
}

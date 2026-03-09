import { Plus, Leaf, Bird } from 'lucide-react'

import { useProjectStore } from '@/stores/project-store'
import { useTranslation } from '@/hooks/use-translation'

interface BookShelfProps {
  onOpenProject: (path: string) => void
  onCreateProject: () => void
  onProjectContextMenu?: (path: string, event: React.MouseEvent) => void
}

export function BookShelf({ onOpenProject, onCreateProject, onProjectContextMenu }: BookShelfProps) {
  const { t, language } = useTranslation()
  const projectList = useProjectStore((s) => s.projectList)

  const accentColors = [
    { gradient: 'linear-gradient(135deg, #0ea5e9, #2563eb)', icon: Bird },
    { gradient: 'linear-gradient(135deg, #34d399, #10b981)', icon: Leaf },
  ]

  return (
    <div className="bento-card bento-card-library">
      <div className="bento-library-header">
        <div className="bento-library-title">{t('workspace.libraryTitle')}</div>

        <div className="bento-badge-row">
          <span className="bento-badge bento-badge-active">
            {t('workspace.libraryAll')} ({projectList.length})
          </span>
          <span className="bento-badge">{t('workspace.librarySerializing')}</span>
          <span className="bento-badge">{t('workspace.libraryFinished')}</span>
        </div>
      </div>

      <div className="bento-novel-shelf">
        {projectList.slice(0, 5).map((project, index) => {
          const colorConfig = accentColors[index % accentColors.length]
          const IconComponent = colorConfig.icon

          return (
            <button
              key={project.path}
              className="bento-book-item"
              onClick={() => onOpenProject(project.path)}
              onContextMenu={(event) => onProjectContextMenu?.(project.path, event)}
            >
              <div className="bento-book-cover" style={{ background: colorConfig.gradient }}>
                {project.coverImage ? (
                  <img src={project.coverImage} alt={project.name} className="bento-book-cover-image" />
                ) : (
                  <IconComponent size={32} className="bento-book-cover-icon" />
                )}
              </div>
              <h3 className="bento-book-title">{project.name}</h3>
              <div className="bento-book-meta">
                {t('workspace.lastUpdated')} {new Date(project.lastOpenedAt).toLocaleDateString(language === 'zh' ? 'zh-CN' : 'en-US')}
              </div>
            </button>
          )
        })}

        <button className="bento-book-item" onClick={onCreateProject}>
          <div className="bento-book-cover bento-book-cover-create">
            <Plus size={24} className="bento-book-create-icon" />
          </div>
          <h3 className="bento-book-title bento-book-title-create">{t('workspace.newWorldTitle')}</h3>
          <div className="bento-book-meta">{t('workspace.newWorldSubtitle')}</div>
        </button>
      </div>
    </div>
  )
}

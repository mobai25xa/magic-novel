import { BookOpen, FileText, Sparkles, ListTree, ArrowRight } from 'lucide-react'

import { useProjectStore } from '@/stores/project-store'
import { useTranslation } from '@/hooks/use-translation'

interface HeroCardProps {
  onOpenProject: (path: string) => void
}

export function HeroCard({ onOpenProject }: HeroCardProps) {
  const { t } = useTranslation()
  const projectList = useProjectStore((s) => s.projectList)
  const latestProject = projectList[0] ?? null

  return (
    <div className="bento-card bento-card-hero">
      <div className="bento-hero-info">
        <div className="bento-hero-greeting">
          <Sparkles size={16} className="bento-sparkle" style={{ color: 'var(--text-success-dark)' }} />
          {t('workspace.greeting')}
        </div>

        {latestProject ? (
          <>
            <h1 className="bento-hero-title">{t('workspace.continueWriting')}《{latestProject.name}》</h1>
            <p className="bento-hero-subtitle">{t('workspace.continueWritingHint')}</p>

            <div className="bento-hero-actions">
              <button className="bento-create-btn bento-hero-primary" onClick={() => onOpenProject(latestProject.path)}>
                <span className="bento-hero-primary-label">{t('workspace.enterEditor')}</span>
                <ArrowRight size={18} />
              </button>
              <button className="icon-btn bento-hero-secondary" title={t('workspace.viewOutline')}>
                <ListTree size={20} />
              </button>
            </div>
          </>
        ) : (
          <>
            <h1 className="bento-hero-title">{t('workspace.welcomeTitle')}</h1>
            <p className="bento-hero-subtitle">{t('workspace.welcomeSubtitle')}</p>
          </>
        )}
      </div>

      {latestProject ? (
        <div className="bento-hero-cover">
          {latestProject.coverImage ? (
            <img src={latestProject.coverImage} alt={latestProject.name} className="bento-hero-cover-image" />
          ) : (
            <>
              <FileText size={64} className="bento-hero-cover-icon" />
              <div className="bento-hero-cover-meta">
                <div className="bento-hero-cover-title">{latestProject.name}</div>
                <div className="bento-hero-cover-subtitle">Vol. 1 Abyss</div>
              </div>
            </>
          )}
        </div>
      ) : (
        <div className="bento-hero-cover bento-hero-cover-empty">
          <BookOpen size={56} className="bento-hero-cover-icon" />
        </div>
      )}
    </div>
  )
}

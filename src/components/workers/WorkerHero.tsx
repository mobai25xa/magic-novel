import { Network, Database, Globe } from 'lucide-react'
import { useTranslation } from '@/hooks/use-translation'

interface WorkerHeroProps {
  workerCount: number
  onCreate?: () => void
  onImport?: () => void
  onManage?: () => void
}

export function WorkerHero({ workerCount, onCreate, onImport, onManage }: WorkerHeroProps) {
  const { translations } = useTranslation()
  const wp = translations.workersPage

  return (
    <div className="bento-card card-hero-workers">
      <div className="workers-hero-info">
        <h1 className="workers-hero-title">{wp.heroTitle}</h1>
        <p className="workers-hero-subtitle">{wp.heroSubtitle}</p>
        <p className="text-xs text-muted-foreground mt-2">{workerCount}</p>
        <div className="worker-actions mt-4" style={{ borderTop: 'none', paddingTop: 0 }}>
          <button type="button" className="btn btn-create" onClick={onCreate}>{wp.createWorker}</button>
          <button type="button" className="btn btn-default" onClick={onImport}>{wp.import}</button>
          <button type="button" className="btn btn-default" onClick={onManage}>{wp.manage}</button>
        </div>
      </div>
      <div className="workers-hero-visual" aria-hidden="true">
        <div className="floating-node node-1">
          <Network size={24} />
        </div>
        <div className="floating-node node-2">
          <Database size={24} />
        </div>
        <div className="floating-node node-3">
          <Globe size={24} />
        </div>
      </div>
    </div>
  )
}

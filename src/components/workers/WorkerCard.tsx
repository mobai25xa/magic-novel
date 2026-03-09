import { Settings2, Wrench } from 'lucide-react'
import { useTranslation } from '@/hooks/use-translation'
import type { WorkerCardData } from './types'

interface WorkerCardProps {
  worker: WorkerCardData
  onConfigure?: () => void
  onRun?: () => void
}

export function WorkerCard({ worker, onConfigure, onRun }: WorkerCardProps) {
  const { translations } = useTranslation()
  const wp = translations.workersPage
  const Icon = worker.icon
  const ActionIcon = worker.primaryActionIcon
  const statusClass = worker.status === 'running' ? 'status-running' : 'status-idle'
  const isPrimary = worker.status === 'running'

  return (
    <div className="bento-card worker-card span-6">
      <div className="worker-header">
        <div className="worker-info">
          <div className={`worker-icon ${worker.colorClass}`}>
            <Icon size={28} />
          </div>
          <div>
            <div className="worker-title">{worker.title}</div>
            <div className="worker-sub">{worker.subtitle}</div>
          </div>
        </div>
        <div
          className={`status-badge ${statusClass}`}
          role="status"
          aria-label={`状态: ${worker.statusLabel}`}
        >
          <div className="status-dot" />
          {worker.statusLabel}
        </div>
      </div>

      <div className="worker-prompt">
        <p className="prompt-text">"{worker.systemPrompt}"</p>
      </div>

      <div className="tool-section">
        <div className="tool-title">
          <Wrench size={14} />
          {wp.equippedTools} ({worker.tools.length})
        </div>
        <div className="tool-pills">
          {worker.tools.map((tool) => {
            const ToolIcon = tool.icon
            return (
              <div className="tool-pill" key={tool.name}>
                <ToolIcon size={14} />
                {tool.name}
              </div>
            )
          })}
        </div>
      </div>

      <div className="worker-actions">
        <button
          type="button"
          className="btn btn-default"
          onClick={onConfigure}
          aria-label={`配置 ${worker.title}`}
        >
          <Settings2 size={16} />
          {wp.configure}
        </button>
        <button
          type="button"
          className={`btn ${isPrimary ? 'btn-create' : 'btn-default'}`}
          onClick={onRun}
          aria-label={`${worker.primaryAction} ${worker.title}`}
        >
          <ActionIcon size={16} />
          {worker.primaryAction}
        </button>
      </div>
    </div>
  )
}

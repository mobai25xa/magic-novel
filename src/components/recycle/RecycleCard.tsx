import { BookOpen, FileText, Layers, Clock, AlertCircle, Calendar, RotateCcw, X, Folder } from 'lucide-react'
import type { RecycleItem, RecycleItemType } from './types'
import { getSeverity } from './types'

const TYPE_ICONS: Record<RecycleItemType, typeof BookOpen> = {
  novel: BookOpen,
  chapter: FileText,
  volume: Layers,
}

interface RecycleCardProps {
  item: RecycleItem
  tw: Record<string, string>
  onRestore: (item: RecycleItem) => void
  onDelete: (item: RecycleItem) => void
}

export function RecycleCard({ item, tw, onRestore, onDelete }: RecycleCardProps) {
  const TypeIcon = TYPE_ICONS[item.type]
  const severity = getSeverity(item.daysRemaining)
  const CountdownIcon = severity === 'urgent' ? AlertCircle : Clock
  const deletedDaysAgo = 30 - item.daysRemaining

  return (
    <div className="bento-card recycle-card span-4">
      <div className="recycle-header">
        <div className={`recycle-icon-wrapper type-${item.type}`}>
          <TypeIcon size={24} />
        </div>
        <div className={`recycle-countdown countdown-${severity}`}>
          <CountdownIcon size={12} />
          {tw.daysRemaining.replace('{days}', String(item.daysRemaining))}
        </div>
      </div>
      <div className="recycle-content">
        <h3>{item.name}</h3>
        <div className="recycle-origin">
          <Folder size={12} />
          {tw.originFrom.replace('{name}', item.origin)}
        </div>
        <p>{item.description}</p>
      </div>
      <div className="recycle-meta">
        <div className="recycle-time">
          <Calendar size={14} />
          {tw.deletedAgo.replace('{days}', String(deletedDaysAgo))}
        </div>
        <div className="recycle-actions">
          <button className="btn-restore" onClick={() => onRestore(item)}>
            <RotateCcw size={12} />
            {tw.restore}
          </button>
          <button
            className="btn-delete-permanent"
            onClick={() => onDelete(item)}
            title={tw.deletePermanent}
          >
            <X size={14} />
          </button>
        </div>
      </div>
    </div>
  )
}

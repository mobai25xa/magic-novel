import { Trash2 } from 'lucide-react'

interface RecycleEmptyStateProps {
  tw: Record<string, string>
  isFilterResult: boolean
}

export function RecycleEmptyState({ tw, isFilterResult }: RecycleEmptyStateProps) {
  if (isFilterResult) {
    return (
      <div className="bento-empty-state span-12">
        <p>{tw.emptyFilter}</p>
      </div>
    )
  }

  return (
    <div className="bento-empty-state span-12">
      <div className="bento-empty-icon bento-empty-icon-muted">
        <Trash2 size={40} />
      </div>
      <p>{tw.empty}</p>
    </div>
  )
}

import { Zap, Plus } from 'lucide-react'

interface SkillEmptyStateProps {
  tw: Record<string, string>
  isFilterResult: boolean
  onCreate: () => void
}

export function SkillEmptyState({ tw, isFilterResult, onCreate }: SkillEmptyStateProps) {
  if (isFilterResult) {
    return (
      <div className="bento-empty-state span-12">
        <p>{tw.emptyFilter}</p>
      </div>
    )
  }

  return (
    <div className="bento-empty-state span-12">
      <div className="bento-empty-icon">
        <Zap size={40} />
      </div>
      <p>{tw.empty}</p>
      <button className="btn btn-solid-success" onClick={onCreate}>
        <Plus size={16} />
        {tw.createSkill}
      </button>
    </div>
  )
}

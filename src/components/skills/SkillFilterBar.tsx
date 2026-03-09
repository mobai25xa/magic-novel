import { Layers, PenTool, BrainCircuit, UserCheck, ShoppingBag, Upload } from 'lucide-react'
import type { SkillCategory } from './types'

interface SkillFilterBarProps {
  tw: Record<string, string>
  active: SkillCategory
  onChange: (cat: SkillCategory) => void
  onOpenImport?: () => void
  onOpenMarket?: () => void
}

const FILTERS: { key: SkillCategory; icon: typeof Layers; labelKey: string }[] = [
  { key: 'all', icon: Layers, labelKey: 'filterAll' },
  { key: 'polish', icon: PenTool, labelKey: 'filterPolish' },
  { key: 'logic', icon: BrainCircuit, labelKey: 'filterLogic' },
  { key: 'roleplay', icon: UserCheck, labelKey: 'filterRoleplay' },
]

export function SkillFilterBar({ tw, active, onChange, onOpenImport, onOpenMarket }: SkillFilterBarProps) {
  return (
    <div className="skills-filter-bar span-12">
      {FILTERS.map((f) => (
        <button
          key={f.key}
          className={`skills-filter-btn${active === f.key ? ' active' : ''}`}
          onClick={() => onChange(f.key)}
        >
          <f.icon size={16} />
          {tw[f.labelKey]}
        </button>
      ))}
      <button className="skills-filter-btn" onClick={onOpenImport}>
        <Upload size={16} />
        {tw.importSkill}
      </button>
      <button className="skills-filter-btn skills-filter-btn-market" onClick={onOpenMarket}>
        <ShoppingBag size={16} />
        {tw.filterMarket}
      </button>
    </div>
  )
}

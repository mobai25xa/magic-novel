import React from 'react'
import { Sparkles, MoreHorizontal, Edit3, Download, Trash2 } from 'lucide-react'

import { Toggle, ContextMenuItem, ContextMenuSeparator } from '@/magic-ui/components'

import { CoordinateContextMenu } from '@/components/common/CoordinateContextMenu'
import type { SkillColorVariant } from './types'
import type { SkillDefinition } from '@/features/global-config'

interface SkillsWorkshopTexts {
  tagBuiltin: string
  tagUser: string
  tagFrequent: string
  tagGlobal: string
  tagExpensive: string
  enabled: string
  disabled: string
  editSkill: string
  exportSkill: string
  deleteSkill: string
}

interface SkillCardProps {
  skill: SkillDefinition
  colorVariant: SkillColorVariant
  tw: SkillsWorkshopTexts
  onToggle: (name: string, enabled: boolean) => void
  onEdit?: (skill: SkillDefinition) => void
  onExport?: (skill: SkillDefinition) => void
  onDelete?: (skill: SkillDefinition) => void
}

function getSkillTags(skill: SkillDefinition, tw: SkillsWorkshopTexts, colorVariant: SkillColorVariant) {
  const tags: { label: string; className?: string }[] = [
    {
      label: skill.source === 'builtin' ? tw.tagBuiltin : tw.tagUser,
      className: skill.source === 'user' ? 'skill-tag-user' : undefined,
    },
  ]

  if (skill.enabled && colorVariant === 1) {
    tags.push({ label: tw.tagFrequent, className: 'skill-tag-frequent' })
  }

  if (!skill.enabled && colorVariant === 3) {
    tags.push({ label: tw.tagGlobal })
    tags.push({ label: tw.tagExpensive })
  }

  return tags
}

export function SkillCard({ skill, colorVariant, tw, onToggle, onEdit, onExport, onDelete }: SkillCardProps) {
  const tags = getSkillTags(skill, tw, colorVariant)
  const [menu, setMenu] = React.useState<{ x: number; y: number } | null>(null)

  return (
    <>
      <div className="bento-card skill-card span-4">
        <div className="skill-header">
          <div className={`skill-icon-wrapper skill-color-${colorVariant}`}>
            <Sparkles size={24} />
          </div>
          <Toggle
            checked={skill.enabled}
            onChange={(e) => {
              e.stopPropagation()
              onToggle(skill.name, e.target.checked)
            }}
            onClick={(e) => e.stopPropagation()}
            aria-label={skill.enabled ? tw.enabled : tw.disabled}
          />
        </div>
        <div className="skill-content">
          <h3>{skill.display_name}</h3>
          <p>{skill.description}</p>
        </div>
        <div className="skill-meta">
          <div className="skill-tags">
            {tags.map((tag) => (
              <span key={`${skill.name}-${tag.label}`} className={`skill-tag${tag.className ? ` ${tag.className}` : ''}`}>
                {tag.label}
              </span>
            ))}
          </div>
          <button
            className="skill-more-btn"
            onClick={(e) => {
              e.preventDefault()
              e.stopPropagation()
              setMenu({ x: e.clientX, y: e.clientY })
            }}
          >
            <MoreHorizontal size={16} />
          </button>
        </div>
      </div>

      {menu ? (
        <CoordinateContextMenu x={menu.x} y={menu.y} onClose={() => setMenu(null)} contentClassName="w-52">
          <ContextMenuItem
            onClick={() => {
              setMenu(null)
              onEdit?.(skill)
            }}
          >
            <Edit3 className="mr-2 h-4 w-4" />
            {tw.editSkill}
          </ContextMenuItem>
          <ContextMenuItem
            onClick={() => {
              setMenu(null)
              onExport?.(skill)
            }}
          >
            <Download className="mr-2 h-4 w-4" />
            {tw.exportSkill}
          </ContextMenuItem>
          <ContextMenuSeparator />
          <ContextMenuItem
            onClick={() => {
              setMenu(null)
              onDelete?.(skill)
            }}
            destructive
          >
            <Trash2 className="mr-2 h-4 w-4" />
            {tw.deleteSkill}
          </ContextMenuItem>
        </CoordinateContextMenu>
      ) : null}
    </>
  )
}

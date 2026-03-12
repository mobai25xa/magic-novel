import { useMemo } from 'react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, ShowMore } from '@/magic-ui/components'

import { CopyPill } from './copy-pill'

export type KnowledgeConflictLike = {
  type: string
  message: string
  item_id?: string
  target_ref?: string
}

export type KnowledgeConflictListProps = {
  conflicts: KnowledgeConflictLike[]
  className?: string
}

function resolveConflictColor(type: string): BadgeProps['color'] {
  const normalized = String(type ?? '').toUpperCase()
  if (normalized.includes('INVALID') || normalized.includes('REVIEW_BLOCKED') || normalized.includes('SOURCE_MISSING')) {
    return 'error'
  }
  return 'warning'
}

export function KnowledgeConflictList({ conflicts, className }: KnowledgeConflictListProps) {
  const entries = useMemo(
    () => (Array.isArray(conflicts) ? conflicts.filter((c): c is KnowledgeConflictLike => Boolean(c && c.type && c.message)) : []),
    [conflicts],
  )

  if (entries.length === 0) {
    return null
  }

  return (
    <div
      className={cn(
        'rounded-md border border-border/60 bg-warning/10 px-2.5 py-2 text-xs space-y-2',
        className,
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <span className="font-medium">Conflicts</span>
          <Badge color="warning" variant="soft" size="sm">
            {entries.length}
          </Badge>
        </div>
      </div>

      <div className="space-y-2">
        {entries.map((conflict, idx) => (
          <div
            key={`${conflict.type}-${conflict.item_id ?? conflict.target_ref ?? idx}`}
            className="rounded-md border border-border/60 bg-background px-2.5 py-2"
          >
            <div className="flex flex-wrap items-center gap-1.5">
              <Badge
                color={resolveConflictColor(conflict.type)}
                variant="soft"
                size="sm"
                className="font-mono"
              >
                {conflict.type}
              </Badge>
              {conflict.item_id ? <CopyPill value={conflict.item_id} title="Copy item_id" /> : null}
              {conflict.target_ref ? <CopyPill value={conflict.target_ref} title="Copy target_ref" /> : null}
            </div>

            <ShowMore maxLines={4} className="mt-1">
              <div className="whitespace-pre-wrap break-words leading-relaxed text-muted-foreground">
                {conflict.message}
              </div>
            </ShowMore>
          </div>
        ))}
      </div>
    </div>
  )
}

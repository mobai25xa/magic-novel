import { useMemo } from 'react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, Button } from '@/magic-ui/components'

import { CopyPill } from './copy-pill'

export type KnowledgeDeltaLike = {
  schema_version?: number
  knowledge_delta_id: string
  status: string
  generated_at?: number | string
  applied_at?: number | string
  accepted_item_ids?: string[]
  rejected_item_ids?: string[]
  conflicts?: Array<{ type: string; message: string }>
  rollback?: { kind?: string; token?: string }
}

export type KnowledgeApplyStatusCardProps = {
  delta: KnowledgeDeltaLike
  onApply?: () => void
  onRollback?: () => void
  applying?: boolean
  rollingBack?: boolean
  showActions?: boolean
  className?: string
}

function normalizeStatus(value: unknown): 'proposed' | 'accepted' | 'applied' | 'rejected' | 'unknown' {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'proposed') return 'proposed'
  if (normalized === 'accepted') return 'accepted'
  if (normalized === 'applied') return 'applied'
  if (normalized === 'rejected') return 'rejected'
  return 'unknown'
}

function resolveStatusColor(status: ReturnType<typeof normalizeStatus>): BadgeProps['color'] {
  switch (status) {
    case 'applied':
      return 'success'
    case 'accepted':
      return 'info'
    case 'rejected':
      return 'error'
    case 'proposed':
      return 'default'
    default:
      return 'default'
  }
}

function formatTime(value: number | string | undefined) {
  if (value === null || value === undefined) {
    return null
  }

  try {
    const date = typeof value === 'number' ? new Date(value) : new Date(String(value))
    if (Number.isNaN(date.getTime())) {
      return null
    }
    return date.toLocaleString()
  } catch {
    return null
  }
}

export function KnowledgeApplyStatusCard({
  delta,
  onApply,
  onRollback,
  applying,
  rollingBack,
  showActions = true,
  className,
}: KnowledgeApplyStatusCardProps) {
  const status = normalizeStatus(delta?.status)
  const appliedAt = formatTime(delta?.applied_at)

  const acceptedCount = useMemo(
    () => (Array.isArray(delta?.accepted_item_ids) ? delta.accepted_item_ids.length : 0),
    [delta],
  )

  const rejectedCount = useMemo(
    () => (Array.isArray(delta?.rejected_item_ids) ? delta.rejected_item_ids.length : 0),
    [delta],
  )

  const conflictCount = useMemo(
    () => (Array.isArray(delta?.conflicts) ? delta.conflicts.length : 0),
    [delta],
  )

  const rollbackToken = delta?.rollback?.token
  const canApply = Boolean(onApply) && !applying && status === 'accepted' && conflictCount === 0
  const canRollback = Boolean(onRollback) && !rollingBack && status === 'applied'

  return (
    <div
      className={cn(
        'rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs space-y-2',
        className,
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">Apply</span>
            <Badge color={resolveStatusColor(status)} variant="soft" size="sm">
              {status}
            </Badge>
            <span className="text-[11px] text-muted-foreground">
              {`accepted ${acceptedCount}`}
              {rejectedCount > 0 ? ` · rejected ${rejectedCount}` : ''}
              {conflictCount > 0 ? ` · conflicts ${conflictCount}` : ''}
            </span>
          </div>

          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            <CopyPill value={delta.knowledge_delta_id} title="Copy delta_id" />
            {rollbackToken ? <CopyPill value={rollbackToken} title="Copy rollback token" /> : null}
          </div>
        </div>

        {appliedAt ? (
          <span className="shrink-0 text-[11px] text-muted-foreground" title={String(delta.applied_at)}>
            {appliedAt}
          </span>
        ) : null}
      </div>

      {showActions ? (
        <div className="flex gap-2">
          <Button
            type="button"
            size="sm"
            className="flex-1 text-xs font-medium disabled:opacity-50"
            onClick={onApply}
            disabled={!canApply}
            title={conflictCount > 0 ? 'Resolve conflicts before applying' : undefined}
          >
            {applying ? 'Applying…' : 'Apply'}
          </Button>

          <Button
            type="button"
            size="sm"
            variant="outline"
            className="flex-1 text-xs font-medium disabled:opacity-50"
            onClick={onRollback}
            disabled={!canRollback}
            title={rollbackToken ? 'Rollback is available' : 'No rollback token'}
          >
            {rollingBack ? 'Rolling back…' : 'Rollback'}
          </Button>
        </div>
      ) : null}
    </div>
  )
}

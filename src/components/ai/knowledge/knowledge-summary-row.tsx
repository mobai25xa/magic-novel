import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge } from '@/magic-ui/components'

export type KnowledgeSummaryStatus = 'proposed' | 'blocked' | 'accepted' | 'applied' | 'rejected' | 'unknown'

export type KnowledgeSummaryRowProps = {
  status: KnowledgeSummaryStatus
  items?: number
  conflicts?: number
  accepted?: number
  rejected?: number
  generatedAt?: number | string
  appliedAt?: number | string
  className?: string
}

function resolveStatusColor(status: KnowledgeSummaryStatus): BadgeProps['color'] {
  switch (status) {
    case 'applied':
      return 'success'
    case 'accepted':
    case 'proposed':
      return 'info'
    case 'blocked':
      return 'warning'
    case 'rejected':
      return 'error'
    default:
      return 'default'
  }
}

function formatTime(value: KnowledgeSummaryRowProps['generatedAt']) {
  if (value === null || value === undefined) {
    return null
  }

  try {
    const date = typeof value === 'number' ? new Date(value) : new Date(String(value))
    if (Number.isNaN(date.getTime())) {
      return null
    }
    return date.toLocaleTimeString()
  } catch {
    return null
  }
}

export function KnowledgeSummaryRow({
  status,
  items,
  conflicts,
  accepted,
  rejected,
  generatedAt,
  appliedAt,
  className,
}: KnowledgeSummaryRowProps) {
  const parts: string[] = []
  if (typeof items === 'number') parts.push(`items ${items}`)
  if (typeof accepted === 'number') parts.push(`accepted ${accepted}`)
  if (typeof rejected === 'number') parts.push(`rejected ${rejected}`)
  if (typeof conflicts === 'number') parts.push(`conflicts ${conflicts}`)

  const time = status === 'applied' ? formatTime(appliedAt) : formatTime(generatedAt)
  if (time) parts.push(time)

  return (
    <div
      className={cn(
        'flex items-center justify-between gap-2 rounded-md border border-border/60 bg-muted/20 px-2.5 py-1.5 text-xs',
        className,
      )}
    >
      <div className="flex min-w-0 items-center gap-2">
        <span className="font-medium">Knowledge</span>
        <Badge color={resolveStatusColor(status)} variant="soft" size="sm">
          {status}
        </Badge>

        {(conflicts ?? 0) > 0 ? (
          <Badge color="warning" variant="outline" size="sm" title="conflicts">
            {`conflicts ${conflicts}`}
          </Badge>
        ) : null}
      </div>

      {parts.length > 0 ? (
        <span className="truncate text-muted-foreground">{parts.join(' · ')}</span>
      ) : null}
    </div>
  )
}

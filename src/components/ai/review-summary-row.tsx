import { cn } from '@/lib/utils'

import { Badge } from '@/magic-ui/components'

export type ReviewSummaryRowProps = {
  status: 'pass' | 'warn' | 'block' | 'unknown'
  warnings?: number
  issues?: number
  className?: string
}

function resolveColor(status: ReviewSummaryRowProps['status']) {
  switch (status) {
    case 'pass':
      return 'success'
    case 'warn':
      return 'warning'
    case 'block':
      return 'error'
    default:
      return 'default'
  }
}

export function ReviewSummaryRow({ status, warnings, issues, className }: ReviewSummaryRowProps) {
  const parts: string[] = []
  if (typeof warnings === 'number') parts.push(`warnings ${warnings}`)
  if (typeof issues === 'number') parts.push(`issues ${issues}`)

  return (
    <div
      className={cn(
        'flex items-center justify-between gap-2 rounded-md border border-border/60 bg-muted/20 px-2.5 py-1.5 text-xs',
        className,
      )}
    >
      <div className="flex min-w-0 items-center gap-2">
        <span className="font-medium">Review</span>
        <Badge color={resolveColor(status)} variant="soft" size="sm">
          {status}
        </Badge>
      </div>

      {parts.length > 0 ? (
        <span className="truncate text-muted-foreground">{parts.join(' · ')}</span>
      ) : (
        <span className="text-muted-foreground">(placeholder)</span>
      )}
    </div>
  )
}

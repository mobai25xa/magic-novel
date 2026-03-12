import { cn } from '@/lib/utils'

import { Badge } from '@/magic-ui/components'

export type ReviewRecommendedAction = 'accept' | 'revise' | 'escalate'

export type ReviewSummaryRowProps = {
  status: 'pass' | 'warn' | 'block' | 'unknown'
  warnings?: number
  issues?: number
  recommendedAction?: ReviewRecommendedAction
  generatedAt?: number | string
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

function resolveActionColor(action: ReviewRecommendedAction) {
  switch (action) {
    case 'accept':
      return 'success'
    case 'revise':
      return 'warning'
    case 'escalate':
      return 'error'
    default:
      return 'default'
  }
}

function formatTime(value: ReviewSummaryRowProps['generatedAt']) {
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

export function ReviewSummaryRow({ status, warnings, issues, recommendedAction, generatedAt, className }: ReviewSummaryRowProps) {
  const parts: string[] = []
  if (typeof issues === 'number') parts.push(`issues ${issues}`)
  if (typeof warnings === 'number') parts.push(`warnings ${warnings}`)
  const time = formatTime(generatedAt)
  if (time) parts.push(time)

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

        {recommendedAction ? (
          <Badge
            color={resolveActionColor(recommendedAction)}
            variant="outline"
            size="sm"
            title="recommended action"
          >
            {recommendedAction}
          </Badge>
        ) : null}
      </div>

      {parts.length > 0 ? (
        <span className="truncate text-muted-foreground">{parts.join(' · ')}</span>
      ) : null}
    </div>
  )
}

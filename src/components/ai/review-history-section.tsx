import { useMemo, useState } from 'react'

import { cn } from '@/lib/utils'

import { ReviewSummaryRow } from './review-summary-row'
import { ReviewReportCard, type ReviewReportLike } from './review-report-card'
import {
  countReviewWarnings,
  normalizeReviewRecommendedAction,
  normalizeReviewStatus,
} from './review-ui-utils'

export type ReviewHistorySectionProps = {
  reports: ReviewReportLike[]
  maxItems?: number
  className?: string
}

export function ReviewHistorySection({ reports, maxItems = 5, className }: ReviewHistorySectionProps) {
  const items = useMemo(() => reports.slice(0, Math.max(0, maxItems)), [reports, maxItems])
  const [open, setOpen] = useState(false)

  if (items.length === 0) {
    return null
  }

  return (
    <details
      className={cn('rounded-md border border-border/60 bg-background-50 px-2.5 py-2', className)}
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Review history (${items.length})`}
      </summary>

      <div className="mt-2 space-y-2">
        {items.map((report, index) => {
          const status = normalizeReviewStatus(report.overall_status)
          const issuesCount = Array.isArray(report.issues) ? report.issues.length : undefined
          const warnings = countReviewWarnings(report)
          const recommendedAction = normalizeReviewRecommendedAction(report.recommended_action)
          const generatedAt = report.generated_at

          return (
            <details
              key={`${String(report.generated_at ?? '')}-${index}`}
              className="rounded-md border border-border/60 bg-background px-2.5 py-2"
            >
              <summary className="cursor-pointer select-none">
                <ReviewSummaryRow
                  status={status}
                  issues={issuesCount}
                  warnings={warnings}
                  recommendedAction={recommendedAction}
                  generatedAt={generatedAt}
                  className="border-0 bg-transparent px-0 py-0"
                />
              </summary>

              <div className="mt-2">
                <ReviewReportCard report={report} />
              </div>
            </details>
          )
        })}
      </div>
    </details>
  )
}

import { useMemo, useState } from 'react'

import { cn } from '@/lib/utils'
import { Badge, Button, Spinner } from '@/magic-ui/components'

import { ReviewSummaryRow } from './review-summary-row'
import { ReviewReportCard, type ReviewReportLike } from './review-report-card'
import { ReviewHistorySection } from './review-history-section'
import {
  normalizeReviewRecommendedAction,
  normalizeReviewStatus,
  countReviewWarnings,
} from './review-ui-utils'

export type MissionReviewSectionProps = {
  report?: ReviewReportLike | null
  history?: ReviewReportLike[] | null
  historyMaxItems?: number
  showWhenEmpty?: boolean

  fixInProgress?: boolean
  fixAttempt?: number
  fixMaxAttempts?: number
  fixUpdatedAt?: number | string
  fixMessage?: string

  waitingDecision?: boolean
  decisionReason?: string
  decisionUpdatedAt?: number | string

  onFix?: () => void
  onDecide?: () => void
  className?: string
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
    return date.toLocaleTimeString()
  } catch {
    return null
  }
}

function formatAttempts(attempt?: number, max?: number) {
  if (typeof attempt !== 'number' || !Number.isFinite(attempt)) {
    return null
  }
  if (typeof max === 'number' && Number.isFinite(max) && max > 0) {
    return `attempt ${attempt}/${max}`
  }
  return `attempt ${attempt}`
}

export function MissionReviewSection({
  report,
  history,
  historyMaxItems,
  showWhenEmpty = false,
  fixInProgress,
  fixAttempt,
  fixMaxAttempts,
  fixUpdatedAt,
  fixMessage,
  waitingDecision,
  decisionReason,
  decisionUpdatedAt,
  onFix,
  onDecide,
  className,
}: MissionReviewSectionProps) {
  const status = normalizeReviewStatus(report?.overall_status)
  const issuesCount = Array.isArray(report?.issues) ? report!.issues.length : undefined
  const warningsCount = report ? countReviewWarnings(report) : undefined

  const recommendedAction = normalizeReviewRecommendedAction(report?.recommended_action)
  const generatedAt = report?.generated_at

  const defaultOpen = status === 'block'
  const [open, setOpen] = useState(() => defaultOpen)
  const detailsKey = report ? `review:${status}` : 'review:empty'

  const shouldRender = showWhenEmpty || Boolean(report)
  const body = useMemo(() => {
    if (!report) {
      return <div className="text-xs text-muted-foreground">No review report yet.</div>
    }
    return <ReviewReportCard report={report} />
  }, [report])

  const fixTime = formatTime(fixUpdatedAt)
  const decideTime = formatTime(decisionUpdatedAt)
  const attemptLabel = formatAttempts(fixAttempt, fixMaxAttempts)
  const showFixMeta = Boolean(fixInProgress) || Boolean(fixMessage) || attemptLabel !== null
  const showDecisionMeta = Boolean(waitingDecision) || Boolean(decisionReason)

  if (!shouldRender) {
    return null
  }

  return (
    <details
      key={detailsKey}
      className={cn('rounded-md border border-border/60 bg-background-50 px-2.5 py-2', className)}
      open={open}
      onToggle={(event) => setOpen(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none">
        <ReviewSummaryRow
          status={status}
          issues={issuesCount}
          warnings={warningsCount}
          recommendedAction={recommendedAction}
          generatedAt={generatedAt}
          className="border-0 bg-transparent px-0 py-0"
        />
      </summary>

      <div className="mt-2 space-y-2">
        {body}

        {(showFixMeta || showDecisionMeta) ? (
          <div className="space-y-2">
            {showFixMeta ? (
              <div className="rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0">
                    {fixInProgress ? <Spinner size="xs" color="muted" /> : null}
                    <span className="font-medium">Fix loop</span>
                    <Badge
                      color={fixInProgress ? 'info' : 'default'}
                      variant="soft"
                      size="sm"
                    >
                      {fixInProgress ? 'running' : 'ready'}
                    </Badge>
                  </div>

                  {(attemptLabel || fixTime) ? (
                    <span className="shrink-0 text-[11px] text-muted-foreground">
                      {[attemptLabel, fixTime].filter(Boolean).join(' · ')}
                    </span>
                  ) : null}
                </div>

                {fixMessage ? (
                  <div className="mt-1 text-muted-foreground break-words leading-relaxed">
                    {fixMessage}
                  </div>
                ) : null}
              </div>
            ) : null}

            {showDecisionMeta ? (
              <div className="rounded-md border border-border/60 bg-warning/10 px-2.5 py-2 text-xs">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2 min-w-0">
                    <Badge color="warning" variant="soft" size="sm">
                      decision required
                    </Badge>
                    {waitingDecision ? (
                      <span className="truncate text-muted-foreground">waiting for user</span>
                    ) : null}
                  </div>
                  {decideTime ? (
                    <span className="shrink-0 text-[11px] text-muted-foreground" title={String(decisionUpdatedAt)}>
                      {decideTime}
                    </span>
                  ) : null}
                </div>

                {decisionReason ? (
                  <div className="mt-1 text-muted-foreground break-words leading-relaxed">
                    {decisionReason}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null}

        {status === 'block' ? (
          <div className="flex gap-2">
            <Button
              type="button"
              size="sm"
              variant="outline"
              className="flex-1 text-xs font-medium disabled:opacity-50"
              onClick={onFix}
              disabled={!onFix || Boolean(fixInProgress)}
            >
              {fixInProgress ? 'Fixing…' : 'Fix'}
            </Button>
            <Button
              type="button"
              size="sm"
              className="flex-1 text-xs font-medium disabled:opacity-50"
              onClick={onDecide}
              disabled={!onDecide}
            >
              Decide
            </Button>
          </div>
        ) : null}

        {history && history.length > 0 ? (
          <ReviewHistorySection
            reports={history}
            maxItems={historyMaxItems}
          />
        ) : null}
      </div>
    </details>
  )
}

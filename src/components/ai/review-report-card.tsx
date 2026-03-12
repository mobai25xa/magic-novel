import { useCallback, useMemo, useState } from 'react'
import { Check, Copy } from 'lucide-react'

import { cn } from '@/lib/utils'
import type { BadgeProps } from '@/magic-ui/components'
import { Badge, ShowMore } from '@/magic-ui/components'

export type ReviewIssue = {
  review_type?: string
  severity: string
  summary: string
  confidence?: number
  evidence_refs?: string[]
  suggested_fix?: string
  auto_fixable?: boolean
}

export type ReviewReportLike = {
  schema_version?: number
  overall_status: string
  review_types?: string[]
  issues?: ReviewIssue[]
  recommended_action?: string
  generated_at?: number | string
}

const severityOrder = ['block', 'warn', 'info', 'unknown'] as const
type NormalizedSeverity = (typeof severityOrder)[number]

function normalizeOverallStatus(value: unknown): 'pass' | 'warn' | 'block' | 'unknown' {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'pass') return 'pass'
  if (normalized === 'warn' || normalized === 'warning') return 'warn'
  if (normalized === 'block' || normalized === 'error' || normalized === 'fail' || normalized === 'failed') return 'block'
  return 'unknown'
}

function resolveOverallColor(status: ReturnType<typeof normalizeOverallStatus>): BadgeProps['color'] {
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

function resolveActionColor(action: string): BadgeProps['color'] {
  switch (String(action ?? '').trim().toLowerCase()) {
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

function normalizeSeverity(value: unknown): NormalizedSeverity {
  const normalized = String(value ?? '').trim().toLowerCase()
  if (normalized === 'block' || normalized === 'error' || normalized === 'critical') return 'block'
  if (normalized === 'warn' || normalized === 'warning') return 'warn'
  if (normalized === 'info' || normalized === 'note') return 'info'
  return 'unknown'
}

function resolveSeverityColor(severity: NormalizedSeverity): BadgeProps['color'] {
  switch (severity) {
    case 'block':
      return 'error'
    case 'warn':
      return 'warning'
    case 'info':
      return 'info'
    default:
      return 'default'
  }
}

function formatGeneratedAt(value: ReviewReportLike['generated_at']) {
  if (value === null || value === undefined) {
    return null
  }

  try {
    const date = typeof value === 'number' ? new Date(value) : new Date(String(value))
    if (Number.isNaN(date.getTime())) {
      return null
    }
    return {
      title: String(value),
      label: date.toLocaleString(),
    }
  } catch {
    return null
  }
}

function formatConfidence(value: unknown) {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return null
  }
  if (value <= 1) {
    return `${Math.round(Math.max(0, value) * 100)}%`
  }
  if (value <= 100) {
    return `${Math.round(value)}%`
  }
  return null
}

function buildSeverityGroups(issues: ReviewIssue[]) {
  const groups: Record<NormalizedSeverity, ReviewIssue[]> = {
    block: [],
    warn: [],
    info: [],
    unknown: [],
  }

  for (const issue of issues) {
    groups[normalizeSeverity(issue.severity)].push(issue)
  }

  return groups
}

function CopyPill({ value, className }: { value: string; className?: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(value)
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1200)
    } catch {
      // ignore clipboard failures
    }
  }, [value])

  return (
    <button
      type="button"
      onClick={() => { void handleCopy() }}
      className={cn(
        'group inline-flex max-w-full items-center gap-1.5 rounded border border-border/60 bg-background px-2 py-1',
        'font-mono text-[11px] text-muted-foreground hover:text-foreground hover:bg-muted/30 transition-colors',
        className,
      )}
      title="Copy"
    >
      <span className="truncate">{value}</span>
      {copied ? (
        <Check className="h-3 w-3 shrink-0 opacity-70" />
      ) : (
        <Copy className="h-3 w-3 shrink-0 opacity-50 group-hover:opacity-80" />
      )}
    </button>
  )
}

function ReviewIssueCard({ issue }: { issue: ReviewIssue }) {
  const confidence = formatConfidence(issue.confidence)

  return (
    <div className="rounded-md border border-border/60 bg-muted/10 px-2.5 py-2 text-xs">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            {issue.review_type ? (
              <Badge variant="outline" size="sm" className="font-mono">
                {issue.review_type}
              </Badge>
            ) : null}

            {typeof issue.auto_fixable === 'boolean' ? (
              <Badge
                variant="soft"
                size="sm"
                color={issue.auto_fixable ? 'info' : 'warning'}
                title={issue.auto_fixable ? 'auto-fixable' : 'needs decision'}
              >
                {issue.auto_fixable ? 'auto-fixable' : 'needs decision'}
              </Badge>
            ) : null}

            {confidence ? (
              <span className="text-[11px] text-muted-foreground" title={String(issue.confidence)}>
                {`confidence ${confidence}`}
              </span>
            ) : null}
          </div>

          <div className="mt-1 text-foreground leading-relaxed break-words">
            {issue.summary}
          </div>
        </div>
      </div>

      {issue.suggested_fix ? (
        <div className="mt-2">
          <div className="text-[11px] font-medium text-secondary-foreground">Suggested fix</div>
          <ShowMore maxLines={4}>
            <div className="mt-1 whitespace-pre-wrap break-words leading-relaxed text-muted-foreground">
              {issue.suggested_fix}
            </div>
          </ShowMore>
        </div>
      ) : null}

      {issue.evidence_refs && issue.evidence_refs.length > 0 ? (
        <div className="mt-2">
          <div className="text-[11px] font-medium text-secondary-foreground">Evidence</div>
          <div className="mt-1 flex flex-wrap gap-1.5">
            {issue.evidence_refs.map((ref, idx) => (
              <CopyPill key={`${ref}-${idx}`} value={ref} />
            ))}
          </div>
        </div>
      ) : null}
    </div>
  )
}

export type ReviewReportCardProps = {
  report: ReviewReportLike
  className?: string
}

export function ReviewReportCard({ report, className }: ReviewReportCardProps) {
  const overall = normalizeOverallStatus(report.overall_status)
  const issues = useMemo(() => (Array.isArray(report.issues) ? report.issues : []), [report.issues])
  const groups = useMemo(() => buildSeverityGroups(issues), [issues])

  const generatedAt = formatGeneratedAt(report.generated_at)
  const recommendedAction = typeof report.recommended_action === 'string' && report.recommended_action.trim()
    ? report.recommended_action.trim()
    : null

  const reviewTypes = Array.isArray(report.review_types)
    ? report.review_types
      .filter((value): value is string => typeof value === 'string' && value.trim().length > 0)
      .map((v) => v.trim())
    : []

  const warnCount = groups.warn.length
  const blockCount = groups.block.length

  return (
    <div
      className={cn(
        'rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs',
        'space-y-2',
        className,
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">Review</span>
            <Badge color={resolveOverallColor(overall)} variant="soft" size="sm">
              {overall}
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

            {issues.length > 0 ? (
              <span className="text-[11px] text-muted-foreground">
                {`issues ${issues.length}`}
                {blockCount > 0 ? ` · block ${blockCount}` : ''}
                {warnCount > 0 ? ` · warn ${warnCount}` : ''}
              </span>
            ) : (
              <span className="text-[11px] text-muted-foreground">no issues</span>
            )}
          </div>

          {reviewTypes.length > 0 ? (
            <div className="mt-1 flex flex-wrap gap-1.5">
              {reviewTypes.map((t) => (
                <Badge key={t} variant="outline" size="sm" className="font-mono">
                  {t}
                </Badge>
              ))}
            </div>
          ) : null}
        </div>

        {generatedAt ? (
          <span
            className="shrink-0 text-[11px] text-muted-foreground"
            title={generatedAt.title}
          >
            {generatedAt.label}
          </span>
        ) : null}
      </div>

      {issues.length > 0 ? (
        <div className="space-y-2">
          {severityOrder
            .filter((severity) => groups[severity].length > 0)
            .map((severity) => (
              <div key={severity} className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Badge
                      color={resolveSeverityColor(severity)}
                      variant="soft"
                      size="sm"
                    >
                      {severity}
                    </Badge>
                    <span className="text-[11px] text-muted-foreground">
                      {groups[severity].length}
                    </span>
                  </div>
                </div>

                <div className="space-y-2">
                  {groups[severity].map((issue, idx) => (
                    <ReviewIssueCard key={`${issue.review_type ?? 'issue'}-${idx}`} issue={issue} />
                  ))}
                </div>
              </div>
            ))}
        </div>
      ) : null}
    </div>
  )
}

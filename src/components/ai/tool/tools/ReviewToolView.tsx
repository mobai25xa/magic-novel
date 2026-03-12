import { useMemo } from 'react'

import type { AgentUiToolStep } from '@/lib/agent-chat/types'

import { AiToolContent, Badge, CodeBlock, ShowMore } from '@/magic-ui/components'
import { formatPreviewText } from '../tool-view-utils'

type ReviewToolViewProps = {
  step: AgentUiToolStep
}

type ReviewIssuePreview = {
  issue_id?: string
  review_type?: string
  severity?: 'info' | 'warn' | 'block'
  summary?: string
  auto_fixable?: boolean
  evidence_refs?: string[]
  suggested_fix?: string
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

function statusBadge(status: string) {
  if (status === 'pass') return <Badge color="success" variant="soft" size="sm">pass</Badge>
  if (status === 'warn') return <Badge color="warning" variant="soft" size="sm">warn</Badge>
  if (status === 'block') return <Badge color="error" variant="soft" size="sm">block</Badge>
  return <Badge color="default" variant="soft" size="sm">unknown</Badge>
}

function severityBadge(severity?: string) {
  if (severity === 'block') return <Badge color="error" variant="soft" size="sm">block</Badge>
  if (severity === 'warn') return <Badge color="warning" variant="soft" size="sm">warn</Badge>
  if (severity === 'info') return <Badge color="default" variant="soft" size="sm">info</Badge>
  return <Badge color="default" variant="soft" size="sm">—</Badge>
}

function formatMaybeTime(ts?: number | null): string {
  if (!ts || !Number.isFinite(ts)) return '—'
  try {
    return new Date(ts).toLocaleString()
  } catch {
    return String(ts)
  }
}

export function ReviewToolView({ step }: ReviewToolViewProps) {
  const preview = useMemo(() => asRecord(step.outputPreview) ?? null, [step.outputPreview])

  const overall = typeof preview?.overall_status === 'string' ? preview.overall_status : 'unknown'
  const action = typeof preview?.recommended_action === 'string' ? preview.recommended_action : undefined
  const generatedAt = typeof preview?.generated_at === 'number' ? preview.generated_at : undefined
  const issueCounts = asRecord(preview?.issue_counts)
  const issuesTop = Array.isArray(preview?.issues_top)
    ? (preview?.issues_top as ReviewIssuePreview[])
    : Array.isArray(preview?.issues)
      ? (preview?.issues as ReviewIssuePreview[]).slice(0, 12)
      : []

  if (!preview) {
    return (
      <AiToolContent className="space-y-2">
        <div className="text-xs text-muted-foreground">No structured review preview available.</div>
        {step.rawOutput ? (
          <ShowMore maxLines={14}>
            <CodeBlock className="text-xs text-foreground whitespace-pre-wrap break-words leading-relaxed">
              {step.rawOutput}
            </CodeBlock>
          </ShowMore>
        ) : null}
      </AiToolContent>
    )
  }

  return (
    <AiToolContent className="space-y-2">
      <div className="flex items-center gap-2 flex-wrap">
        {statusBadge(overall)}
        {typeof issueCounts?.block === 'number' && issueCounts.block > 0 ? (
          <Badge color="error" variant="soft" size="sm">{issueCounts.block} block</Badge>
        ) : null}
        {typeof issueCounts?.warn === 'number' && issueCounts.warn > 0 ? (
          <Badge color="warning" variant="soft" size="sm">{issueCounts.warn} warn</Badge>
        ) : null}
        {action ? (
          <Badge color="default" variant="soft" size="sm">{action}</Badge>
        ) : null}
      </div>

      <div className="text-[11px] text-muted-foreground">
        generated: <span className="font-mono">{formatMaybeTime(generatedAt)}</span>
      </div>

      {issuesTop.length > 0 ? (
        <div className="space-y-1">
          <div className="text-xs font-medium text-secondary-foreground">Issues (top {issuesTop.length})</div>
          <ShowMore maxLines={18}>
            <div className="space-y-1">
              {issuesTop.map((issue, idx) => (
                <div key={issue.issue_id ?? String(idx)} className="rounded border border-border p-2">
                  <div className="flex items-center gap-2 flex-wrap">
                    {severityBadge(issue.severity)}
                    {issue.review_type ? (
                      <span className="font-mono text-[11px] text-muted-foreground">{issue.review_type}</span>
                    ) : null}
                    {issue.auto_fixable ? (
                      <Badge color="info" variant="soft" size="sm">auto</Badge>
                    ) : null}
                  </div>
                  {issue.summary ? (
                    <p className="text-xs text-muted-foreground mt-1 whitespace-pre-wrap">{issue.summary}</p>
                  ) : null}
                  {issue.suggested_fix ? (
                    <p className="text-xs text-muted-foreground mt-1 whitespace-pre-wrap">
                      <span className="opacity-70">fix:</span> {issue.suggested_fix}
                    </p>
                  ) : null}
                  {issue.evidence_refs?.length ? (
                    <p className="text-[11px] text-muted-foreground mt-1">
                      <span className="opacity-70">evidence:</span>{' '}
                      <span className="font-mono">{issue.evidence_refs.slice(0, 3).join(', ')}</span>
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          </ShowMore>
        </div>
      ) : (
        <div className="text-xs text-muted-foreground">No issues preview.</div>
      )}

      <details className="rounded border border-border px-2 py-1">
        <summary className="cursor-pointer select-none list-none text-xs font-medium text-secondary-foreground">
          Raw preview
        </summary>
        <div className="mt-2">
          <CodeBlock className="text-[11px] text-foreground whitespace-pre-wrap break-words leading-relaxed">
            {formatPreviewText(preview)}
          </CodeBlock>
        </div>
      </details>
    </AiToolContent>
  )
}

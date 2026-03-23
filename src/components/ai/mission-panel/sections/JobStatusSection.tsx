import type { JobPhase, JobStatusView, RecoveryTone } from '../derived'

type JobStatusSectionProps = {
  summary: JobStatusView
}

function phaseClassName(phase: JobPhase) {
  switch (phase) {
    case 'running':
      return 'border-sky-500/30 bg-sky-500/10 text-sky-700'
    case 'waiting':
      return 'border-amber-500/30 bg-amber-500/10 text-amber-700'
    case 'blocked':
      return 'border-orange-500/30 bg-orange-500/10 text-orange-700'
    case 'completed':
      return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700'
    case 'failed':
      return 'border-rose-500/30 bg-rose-500/10 text-rose-700'
    case 'ready':
    default:
      return 'border-border/70 bg-muted/30 text-foreground'
  }
}

function statClassName(input: { accent?: 'running' | 'failed' | 'blocked' }) {
  switch (input.accent) {
    case 'running':
      return 'border-sky-500/25 bg-sky-500/10 text-sky-700'
    case 'failed':
      return 'border-rose-500/25 bg-rose-500/10 text-rose-700'
    case 'blocked':
      return 'border-amber-500/25 bg-amber-500/10 text-amber-700'
    default:
      return 'border-border/60 bg-background text-muted-foreground'
  }
}

function recoveryToneClassName(tone: RecoveryTone | null) {
  switch (tone) {
    case 'success':
      return 'border-emerald-500/25 bg-emerald-500/10 text-emerald-700'
    case 'warning':
      return 'border-amber-500/25 bg-amber-500/10 text-amber-700'
    case 'error':
      return 'border-rose-500/25 bg-rose-500/10 text-rose-700'
    case 'info':
      return 'border-sky-500/25 bg-sky-500/10 text-sky-700'
    default:
      return 'border-border/60 bg-background text-muted-foreground'
  }
}

function formatUpdatedAt(ts: number) {
  if (!Number.isFinite(ts) || ts <= 0) {
    return null
  }

  return new Date(ts).toLocaleTimeString()
}

type CountChipProps = {
  label: string
  value: number
  accent?: 'running' | 'failed' | 'blocked'
}

function CountChip({ label, value, accent }: CountChipProps) {
  if (value <= 0) {
    return null
  }

  return (
    <span className={`rounded-full border px-2 py-0.5 ${statClassName({ accent })}`}>
      <span className="font-medium text-foreground">{value}</span>
      {' '}
      {label}
    </span>
  )
}

export function JobStatusSection({ summary }: JobStatusSectionProps) {
  const updatedAtLabel = formatUpdatedAt(summary.updatedAt)
  const blockerPreview = summary.blockers.slice(0, 2)
  const hiddenBlockerCount = Math.max(0, summary.blockers.length - blockerPreview.length)

  return (
    <section className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
            Job State
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-2">
            <span className={`rounded-full border px-2 py-0.5 text-[11px] font-medium capitalize ${phaseClassName(summary.phase)}`}>
              {summary.phaseLabel}
            </span>
            <span className="rounded-full border border-border/60 bg-background px-2 py-0.5 text-[11px] text-muted-foreground">
              {summary.statusLabel}
            </span>
            {summary.recoveryLabel ? (
              <span className={`rounded-full border px-2 py-0.5 text-[11px] capitalize ${recoveryToneClassName(summary.recoveryTone)}`}>
                {summary.recoveryLabel}
              </span>
            ) : null}
            {summary.blockerCount > 0 ? (
              <span className="rounded-full border border-amber-500/25 bg-amber-500/10 px-2 py-0.5 text-[11px] text-amber-700">
                {`blockers ${summary.blockerCount}`}
              </span>
            ) : null}
          </div>
          <div className="mt-2 text-sm font-medium text-foreground">
            {summary.headline}
          </div>
          {summary.detail ? (
            <div className="mt-1 text-xs text-muted-foreground break-words">
              {summary.detail}
            </div>
          ) : null}
          {summary.recoveryHint && summary.recoveryHint !== summary.detail ? (
            <div className="mt-1 text-xs text-muted-foreground break-words">
              {summary.recoveryHint}
            </div>
          ) : null}
        </div>

        {updatedAtLabel ? (
          <div className="shrink-0 text-[11px] text-muted-foreground">
            {updatedAtLabel}
          </div>
        ) : null}
      </div>

      {summary.hasSnapshot ? (
        <div className="mt-3 flex flex-wrap gap-2 text-[11px]">
          <CountChip label="running" value={summary.runningTaskCount} accent="running" />
          <CountChip label="ready" value={summary.readyTaskCount} />
          <CountChip label="completed" value={summary.completedTaskCount} />
          <CountChip label="failed" value={summary.failedTaskCount} accent="failed" />
          <CountChip label="blocked" value={summary.blockerCount} accent="blocked" />
        </div>
      ) : null}

      {blockerPreview.length > 0 ? (
        <div className="mt-3 space-y-2">
          {blockerPreview.map((blocker) => (
            <div
              key={blocker.blockerId}
              className="rounded-md border border-border/60 bg-background px-2 py-1.5 text-xs"
            >
              <div className="font-medium text-foreground">
                {blocker.kindLabel}
              </div>
              <div className="mt-0.5 text-muted-foreground break-words">
                {blocker.summary}
              </div>
            </div>
          ))}

          {hiddenBlockerCount > 0 ? (
            <div className="text-[11px] text-muted-foreground">
              {`+${hiddenBlockerCount} more blocker${hiddenBlockerCount === 1 ? '' : 's'}`}
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  )
}

import type { JobStatusView, RecoveryEntryView, RecoveryTone } from '../derived'

type RecoverySectionProps = {
  summary: JobStatusView
  open: boolean
  onOpenChange: (open: boolean) => void
}

function toneClassName(tone: RecoveryTone) {
  switch (tone) {
    case 'success':
      return 'border-emerald-500/25 bg-emerald-500/10 text-emerald-700'
    case 'warning':
      return 'border-amber-500/25 bg-amber-500/10 text-amber-700'
    case 'error':
      return 'border-rose-500/25 bg-rose-500/10 text-rose-700'
    case 'info':
    default:
      return 'border-sky-500/25 bg-sky-500/10 text-sky-700'
  }
}

function formatTime(ts: number) {
  if (!Number.isFinite(ts) || ts <= 0) {
    return null
  }

  return new Date(ts).toLocaleTimeString()
}

function RecoveryEntryCard({ entry }: { entry: RecoveryEntryView }) {
  const timeLabel = formatTime(entry.ts)

  return (
    <div className="rounded-md border border-border/60 bg-background px-2 py-1.5 text-xs">
      <div className="flex items-start justify-between gap-2">
        <span className={`rounded-full border px-2 py-0.5 text-[11px] ${toneClassName(entry.tone)}`}>
          {entry.tone}
        </span>
        {timeLabel ? (
          <span className="shrink-0 text-[11px] text-muted-foreground">{timeLabel}</span>
        ) : null}
      </div>
      <div className="mt-1 text-muted-foreground break-words">
        {entry.message}
      </div>
    </div>
  )
}

export function RecoverySection({ summary, open, onOpenChange }: RecoverySectionProps) {
  if (!summary.recoveryHint && summary.recoveryEntries.length === 0) {
    return null
  }

  const entries = summary.recoveryEntries.slice(0, 6)

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={open}
      onToggle={(event) => onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {summary.recoveryEntries.length > 0
          ? `Recovery & Diagnostics (${summary.recoveryEntries.length})`
          : 'Recovery & Diagnostics'}
      </summary>

      {summary.recoveryLabel || summary.recoveryHint ? (
        <div className="mt-2 rounded-md border border-border/60 bg-background px-2 py-1.5 text-xs">
          {summary.recoveryLabel ? (
            <div className="font-medium text-foreground capitalize">
              {summary.recoveryLabel}
            </div>
          ) : null}
          {summary.recoveryHint ? (
            <div className="mt-0.5 text-muted-foreground break-words">
              {summary.recoveryHint}
            </div>
          ) : null}
        </div>
      ) : null}

      {entries.length > 0 ? (
        <div className="mt-2 space-y-2">
          {entries.map((entry) => <RecoveryEntryCard key={entry.key} entry={entry} />)}
        </div>
      ) : null}
    </details>
  )
}

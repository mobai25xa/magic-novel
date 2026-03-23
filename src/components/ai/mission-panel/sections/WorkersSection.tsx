import { WorkerStepCard } from '@/components/ai/worker-step-card'

type WorkerInfo = {
  status: string
  featureId?: string | null
  summary?: string | null
  updatedAt: number
}

type WorkersSectionProps = {
  workerEntries: Array<[string, WorkerInfo]>
  issueCountByWorkerId: Record<string, number>
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function WorkersSection({ workerEntries, issueCountByWorkerId, open, onOpenChange }: WorkersSectionProps) {
  if (workerEntries.length === 0) {
    return null
  }

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={open}
      onToggle={(event) => onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Workers (${workerEntries.length})`}
      </summary>

      <div className="mt-2 space-y-2 max-h-60 overflow-y-auto pr-1">
        {workerEntries.map(([wid, info]) => {
          const issueCount = issueCountByWorkerId[wid] ?? 0
          const summary = issueCount > 0
            ? [info.summary, `issues ${issueCount}`].filter(Boolean).join(' · ')
            : info.summary

          return (
            <WorkerStepCard
              key={wid}
              workerId={wid}
              status={info.status}
              featureId={info.featureId ?? undefined}
              summary={summary ?? undefined}
              updatedAt={info.updatedAt}
            />
          )
        })}
      </div>
    </details>
  )
}


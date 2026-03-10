import { cn } from '@/lib/utils'

import { AiStatusBadge } from './status-badge'

export type WorkerStepCardProps = {
  workerId: string
  status: string
  featureId?: string
  summary?: string
  updatedAt?: number
  className?: string
}

function formatTime(ts?: number) {
  if (!ts || !Number.isFinite(ts)) {
    return null
  }
  try {
    return new Date(ts).toLocaleTimeString()
  } catch {
    return null
  }
}

export function WorkerStepCard({
  workerId,
  status,
  featureId,
  summary,
  updatedAt,
  className,
}: WorkerStepCardProps) {
  const time = formatTime(updatedAt)

  return (
    <div
      className={cn(
        'rounded-md border border-border/60 bg-background px-2.5 py-2 text-xs',
        'space-y-1',
        className,
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="font-mono text-muted-foreground truncate" title={workerId}>
              {workerId}
            </span>
            <AiStatusBadge status={status} />
          </div>
          {featureId ? (
            <div className="mt-0.5 truncate text-muted-foreground" title={featureId}>
              {featureId}
            </div>
          ) : null}
        </div>

        {time ? (
          <span className="shrink-0 text-[11px] text-muted-foreground" title={String(updatedAt)}>
            {time}
          </span>
        ) : null}
      </div>

      {summary ? (
        <div className="text-muted-foreground leading-relaxed break-words">
          {summary}
        </div>
      ) : null}
    </div>
  )
}

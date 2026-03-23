type MissionSummaryCardProps = {
  completedFeatureCount: number
  featuresCount: number
  failedFeatureCount: number
  workerCount: number
  runningWorkersCount: number
  failedWorkersCount: number
  resultCount: number
  failedResultCount: number
  lastProgressMessage?: string | null
}

export function MissionSummaryCard({
  completedFeatureCount,
  featuresCount,
  failedFeatureCount,
  workerCount,
  runningWorkersCount,
  failedWorkersCount,
  resultCount,
  failedResultCount,
  lastProgressMessage,
}: MissionSummaryCardProps) {
  return (
    <div className="rounded-md border border-border/60 bg-muted/20 px-2.5 py-2 text-xs">
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
        <span className="text-muted-foreground">Features</span>
        <span className="font-medium text-foreground">
          {completedFeatureCount}/{featuresCount}
        </span>
        {failedFeatureCount > 0 ? (
          <span className="text-destructive">{`failed ${failedFeatureCount}`}</span>
        ) : null}

        <span className="text-muted-foreground">Workers</span>
        <span className="font-medium text-foreground">{workerCount}</span>
        {runningWorkersCount > 0 ? (
          <span className="text-ai-status-running">{`running ${runningWorkersCount}`}</span>
        ) : null}
        {failedWorkersCount > 0 ? (
          <span className="text-destructive">{`failed ${failedWorkersCount}`}</span>
        ) : null}

        {resultCount > 0 ? (
          <>
            <span className="text-muted-foreground">Results</span>
            <span className="font-medium text-foreground">{resultCount}</span>
            {failedResultCount > 0 ? (
              <span className="text-destructive">{`non-completed ${failedResultCount}`}</span>
            ) : null}
          </>
        ) : null}
      </div>

      {lastProgressMessage ? (
        <div className="mt-1.5 text-muted-foreground truncate" title={lastProgressMessage}>
          {lastProgressMessage}
        </div>
      ) : null}
    </div>
  )
}

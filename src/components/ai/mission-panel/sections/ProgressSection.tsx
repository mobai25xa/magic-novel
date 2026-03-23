import { ProgressLog } from '../components/ProgressLog'

type ProgressSectionProps = {
  progressLog: Array<{ ts: number; message: string }>
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function ProgressSection({ progressLog, open, onOpenChange }: ProgressSectionProps) {
  if (progressLog.length === 0) {
    return null
  }

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={open}
      onToggle={(event) => onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        Progress
      </summary>
      <ProgressLog entries={progressLog} />
    </details>
  )
}


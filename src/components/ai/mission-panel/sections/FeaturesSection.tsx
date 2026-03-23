import { AiStatusBadge } from '@/components/ai/status-badge'

type FeatureEntry = {
  id: string
  status: string
  description: string
  skill?: string | null
}

type FeaturesSectionProps = {
  features: FeatureEntry[]
  completedFeatureCount: number
  open: boolean
  onOpenChange: (open: boolean) => void
}

function iconForStatus(status: string) {
  switch (status) {
    case 'completed':
      return '✓'
    case 'failed':
      return '✗'
    case 'in_progress':
      return '▶'
    case 'cancelled':
      return '–'
    default:
      return '○'
  }
}

export function FeaturesSection({ features, completedFeatureCount, open, onOpenChange }: FeaturesSectionProps) {
  if (features.length === 0) {
    return null
  }

  return (
    <details
      className="rounded-md border border-border/60 bg-background-50 px-2.5 py-2"
      open={open}
      onToggle={(event) => onOpenChange(event.currentTarget.open)}
    >
      <summary className="cursor-pointer select-none text-xs font-medium text-secondary-foreground">
        {`Features (${completedFeatureCount}/${features.length})`}
      </summary>

      <div className="mt-2 space-y-1">
        {features.map((feature) => (
          <div key={feature.id} className="flex items-start gap-2 text-xs py-0.5">
            <span className="mt-0.5">{iconForStatus(feature.status)}</span>
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <span className="opacity-80 truncate block" title={feature.description}>{feature.description}</span>
                <AiStatusBadge status={feature.status} />
              </div>
              {feature.skill ? (
                <div
                  className="mt-0.5 font-mono text-[11px] text-muted-foreground truncate"
                  title={feature.skill}
                >
                  {feature.skill}
                </div>
              ) : null}
            </div>
          </div>
        ))}
      </div>
    </details>
  )
}


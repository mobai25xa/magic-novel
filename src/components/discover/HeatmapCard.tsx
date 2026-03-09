import type { MonthSummary } from '@/features/discover-analytics'
import { WritingHeatmap } from './WritingHeatmap'

interface HeatmapCardProps {
  yearStats: MonthSummary[]
  currentYear: number
  onYearChange: (year: number) => void
}

export function HeatmapCard(props: HeatmapCardProps) {
  return (
    <div className="bento-card card-heatmap span-12 row-2">
      <WritingHeatmap {...props} />
    </div>
  )
}

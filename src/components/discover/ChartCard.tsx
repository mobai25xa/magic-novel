import { TrendingUp } from 'lucide-react'
import type { DailyStats } from '@/features/discover-analytics'
import { useTranslation } from '@/hooks/use-translation'
import { WritingChart } from './WritingChart'

interface ChartCardProps {
  stats: DailyStats[]
}

export function ChartCard({ stats }: ChartCardProps) {
  const { translations } = useTranslation()

  return (
    <div className="bento-card card-chart span-8 row-3">
      <h3 className="card-title">
        <TrendingUp size={18} style={{ color: 'var(--primary)' }} />
        {translations.discover.chartTitle}
      </h3>
      <div className="chart-container">
        <WritingChart stats={stats} />
      </div>
    </div>
  )
}

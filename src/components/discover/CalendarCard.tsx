import type { DailyStats } from '@/features/discover-analytics'
import { WritingCalendar } from './WritingCalendar'

interface CalendarCardProps {
  monthStats: DailyStats[]
  currentYear: number
  currentMonth: number
  onMonthChange: (year: number, month: number) => void
}

export function CalendarCard(props: CalendarCardProps) {
  return (
    <div className="bento-card card-calendar span-4 row-3">
      <WritingCalendar {...props} />
    </div>
  )
}

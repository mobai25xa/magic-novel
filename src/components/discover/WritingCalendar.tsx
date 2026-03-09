import { useMemo } from 'react'
import { useTranslation } from '@/hooks/use-translation'
import type { DailyStats } from '@/features/discover-analytics'
import {
  buildCalendarDays,
  calculateMonthSummary,
  getMonthNames,
  getWeekDays,
} from './writing-calendar-data'
import { WritingCalendarView } from './writing-calendar-view'

interface WritingCalendarProps {
  monthStats: DailyStats[]
  currentYear: number
  currentMonth: number
  onMonthChange: (year: number, month: number) => void
}

export function WritingCalendar({
  monthStats,
  currentYear,
  currentMonth,
  onMonthChange,
}: WritingCalendarProps) {
  const { translations } = useTranslation()

  const weekDays = getWeekDays(translations)
  const monthNames = getMonthNames(translations)

  const calendarDays = useMemo(() => {
    return buildCalendarDays(monthStats, currentYear, currentMonth)
  }, [currentYear, currentMonth, monthStats])

  const monthSummary = useMemo(() => {
    return calculateMonthSummary(monthStats)
  }, [monthStats])

  const handlePrevMonth = () => {
    if (currentMonth === 1) {
      onMonthChange(currentYear - 1, 12)
    } else {
      onMonthChange(currentYear, currentMonth - 1)
    }
  }

  const handleNextMonth = () => {
    if (currentMonth === 12) {
      onMonthChange(currentYear + 1, 1)
    } else {
      onMonthChange(currentYear, currentMonth + 1)
    }
  }

  return (
    <WritingCalendarView
      translations={translations}
      weekDays={weekDays}
      monthNames={monthNames}
      currentYear={currentYear}
      currentMonth={currentMonth}
      calendarDays={calendarDays}
      monthSummary={monthSummary}
      onPrevMonth={handlePrevMonth}
      onNextMonth={handleNextMonth}
    />
  )
}

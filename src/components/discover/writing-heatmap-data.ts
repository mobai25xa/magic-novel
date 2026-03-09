import type { MonthSummary } from '@/features/discover-analytics'

export interface DayCell {
  date: Date
  wordCount: number
  month: number
  year: number
}

export function buildWeeklyGrid(
  yearStats: MonthSummary[],
  year: number,
): DayCell[][] {
  const dateMap = buildDateMap(yearStats)
  const { startDate, endDate } = resolveDateRange(year)

  const weeks: DayCell[][] = []
  let currentWeek: DayCell[] = []
  const currentDate = new Date(startDate)

  while (currentDate <= endDate || currentWeek.length > 0) {
    const dateKey = currentDate.toISOString().split('T')[0]
    const wordCount = dateMap.get(dateKey) || 0

    currentWeek.push({
      date: new Date(currentDate),
      wordCount,
      month: currentDate.getMonth() + 1,
      year: currentDate.getFullYear(),
    })

    if (currentWeek.length === 7) {
      weeks.push(currentWeek)
      currentWeek = []
    }

    currentDate.setDate(currentDate.getDate() + 1)
    if (currentDate > endDate && currentWeek.length === 0) break
  }

  if (currentWeek.length > 0) {
    while (currentWeek.length < 7) {
      currentWeek.push({
        date: new Date(currentDate),
        wordCount: 0,
        month: currentDate.getMonth() + 1,
        year: currentDate.getFullYear(),
      })
      currentDate.setDate(currentDate.getDate() + 1)
    }
    weeks.push(currentWeek)
  }

  return weeks
}

function buildDateMap(yearStats: MonthSummary[]): Map<string, number> {
  const dateMap = new Map<string, number>()

  yearStats.forEach((month) => {
    month.daily_words.forEach((words, dayIndex) => {
      const date = new Date(month.year, month.month - 1, dayIndex + 1)
      const dateKey = date.toISOString().split('T')[0]
      dateMap.set(dateKey, words)
    })
  })

  return dateMap
}

function resolveDateRange(year: number): {
  startDate: Date
  endDate: Date
} {
  const startDate = new Date(year, 0, 1)
  const endDate = new Date(year, 11, 31)

  const startDayOfWeek = startDate.getDay()
  startDate.setDate(startDate.getDate() - startDayOfWeek)

  return { startDate, endDate }
}

import type { DailyStats } from '@/features/discover-analytics'
import type { Translations } from '@/i18n/locales/zh'

export function getWeekDays(translations: Translations): string[] {
  return translations.discover.weekdaysShortSun.split(',')
}

export function getMonthNames(translations: Translations): string[] {
  return translations.discover.monthsShort.split(',')
}

export function buildCalendarDays(
  monthStats: DailyStats[],
  currentYear: number,
  currentMonth: number,
): Array<DailyStats | null> {
  const firstDay = new Date(currentYear, currentMonth - 1, 1)
  const lastDay = new Date(currentYear, currentMonth, 0)
  const daysInMonth = lastDay.getDate()

  const startDayOfWeek = firstDay.getDay()

  const days: Array<DailyStats | null> = []

  for (let i = 0; i < startDayOfWeek; i += 1) {
    days.push(null)
  }

  for (let day = 1; day <= daysInMonth; day += 1) {
    const dateStr = `${currentYear}-${currentMonth.toString().padStart(2, '0')}-${day
      .toString()
      .padStart(2, '0')}`
    const stats = monthStats.find((s) => s.date === dateStr)
    days.push(
      stats || {
        date: dateStr,
        word_count: 0,
        writing_duration_secs: 0,
        thinking_duration_secs: 0,
        sessions: [],
      },
    )
  }

  return days
}

export function calculateMonthSummary(monthStats: DailyStats[]): {
  writingDays: number
  totalWords: number
} {
  return {
    writingDays: monthStats.filter((s) => s.word_count > 0).length,
    totalWords: monthStats.reduce((sum, s) => sum + s.word_count, 0),
  }
}

export function formatDayWordCount(wordCount: number): string {
  if (wordCount <= 0) {
    return ''
  }
  if (wordCount >= 1000) {
    return `${(wordCount / 1000).toFixed(1)}k`
  }
  return String(wordCount)
}

export function isToday(dateStr: string): boolean {
  return dateStr === new Date().toISOString().split('T')[0]
}

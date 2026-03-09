import type { MonthSummary } from '@/features/discover-analytics'

export function calculateMaxStreak(year: MonthSummary[]): number {
  let maxStreak = 0
  let currentStreak = 0

  year.forEach((month) => {
    month.daily_words.forEach((words) => {
      if (words > 0) {
        currentStreak += 1
        maxStreak = Math.max(maxStreak, currentStreak)
      } else {
        currentStreak = 0
      }
    })
  })

  return maxStreak
}

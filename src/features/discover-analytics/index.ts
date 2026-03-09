import {
  loadDiscoverConsecutiveDays,
  loadDiscoverMonthStats,
  loadDiscoverWeekStats,
  loadDiscoverYearStats,
} from '@/features/search-retrieval'
import type { DailyStats, MonthSummary } from '@/lib/tauri-commands'

export type { DailyStats, MonthSummary }

export {
  loadDiscoverWeekStats,
  loadDiscoverMonthStats,
  loadDiscoverYearStats,
  loadDiscoverConsecutiveDays,
}

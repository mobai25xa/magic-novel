import { useCallback, useEffect, useMemo, useState } from 'react'
import {
  loadDiscoverConsecutiveDays,
  loadDiscoverMonthStats,
  loadDiscoverWeekStats,
  loadDiscoverYearStats,
  type DailyStats,
  type MonthSummary,
} from '@/features/discover-analytics'
import { eventBus, EVENTS } from '@/lib/events'

import { calculateMaxStreak } from '@/components/discover/discover-page-stats'

interface DiscoverStatsState {
  weekStats: DailyStats[]
  monthStats: DailyStats[]
  yearStats: MonthSummary[]
  consecutiveDays: number
  maxConsecutiveDays: number
  loading: boolean
  currentYear: number
  currentMonth: number
}

function useDiscoverState() {
  const [weekStats, setWeekStats] = useState<DailyStats[]>([])
  const [monthStats, setMonthStats] = useState<DailyStats[]>([])
  const [yearStats, setYearStats] = useState<MonthSummary[]>([])
  const [consecutiveDays, setConsecutiveDays] = useState(0)
  const [maxConsecutiveDays, setMaxConsecutiveDays] = useState(0)
  const [loading, setLoading] = useState(true)

  const now = new Date()
  const [currentYear, setCurrentYear] = useState(now.getFullYear())
  const [currentMonth, setCurrentMonth] = useState(now.getMonth() + 1)

  const actions = useMemo(
    () => ({
      setWeekStats,
      setMonthStats,
      setYearStats,
      setConsecutiveDays,
      setMaxConsecutiveDays,
      setLoading,
      setCurrentYear,
      setCurrentMonth,
    }),
    [
      setWeekStats,
      setMonthStats,
      setYearStats,
      setConsecutiveDays,
      setMaxConsecutiveDays,
      setLoading,
      setCurrentYear,
      setCurrentMonth,
    ],
  )

  return {
    state: {
      weekStats,
      monthStats,
      yearStats,
      consecutiveDays,
      maxConsecutiveDays,
      loading,
      currentYear,
      currentMonth,
    } satisfies DiscoverStatsState,
    actions,
  }
}

type DiscoverActions = ReturnType<typeof useDiscoverState>['actions']

type DiscoverCoreState = Pick<DiscoverStatsState, 'currentYear' | 'currentMonth'>

function useDiscoverLoaders(
  projectsRootDir: string | null,
  state: DiscoverCoreState,
  actions: DiscoverActions,
) {
  const loadStats = useCallback(async () => {
    if (!projectsRootDir) {
      actions.setLoading(false)
      return
    }

    actions.setLoading(true)
    try {
      const [week, consecutive, year] = await Promise.all([
        loadDiscoverWeekStats(7, projectsRootDir),
        loadDiscoverConsecutiveDays(projectsRootDir),
        loadDiscoverYearStats(state.currentYear, projectsRootDir),
      ])
      actions.setWeekStats(week)
      actions.setConsecutiveDays(consecutive)
      actions.setMaxConsecutiveDays(calculateMaxStreak(year))
      actions.setYearStats(year)
    } catch (error) {
      console.error('Failed to load stats:', error)
    } finally {
      actions.setLoading(false)
    }
  }, [actions, projectsRootDir, state.currentYear])

  const loadMonthStats = useCallback(async () => {
    if (!projectsRootDir) {
      return
    }

    try {
      const month = await loadDiscoverMonthStats(
        state.currentYear,
        state.currentMonth,
        projectsRootDir,
      )
      actions.setMonthStats(month)
    } catch (error) {
      console.error('Failed to load month stats:', error)
    }
  }, [actions, projectsRootDir, state.currentMonth, state.currentYear])

  const handleYearChange = useCallback(async (year: number) => {
    actions.setCurrentYear(year)
    try {
      const yearData = await loadDiscoverYearStats(year, projectsRootDir || undefined)
      actions.setYearStats(yearData)
    } catch (error) {
      console.error('Failed to load year stats:', error)
    }
  }, [actions, projectsRootDir])

  return {
    loadStats,
    loadMonthStats,
    handleYearChange,
  }
}

function useDiscoverRefresh(loadStats: () => Promise<void>, loadMonthStats: () => Promise<void>) {
  useEffect(() => {
    loadStats()
  }, [loadStats])

  useEffect(() => {
    loadMonthStats()
  }, [loadMonthStats])

  useEffect(() => {
    const handleChapterSaved = () => {
      loadStats()
      loadMonthStats()
    }

    eventBus.on(EVENTS.CHAPTER_SAVED, handleChapterSaved)
    return () => {
      eventBus.off(EVENTS.CHAPTER_SAVED, handleChapterSaved)
    }
  }, [loadStats, loadMonthStats])
}

export function useDiscoverStats(projectsRootDir: string | null) {
  const { state, actions } = useDiscoverState()
  const { loadStats, loadMonthStats, handleYearChange } = useDiscoverLoaders(
    projectsRootDir,
    { currentYear: state.currentYear, currentMonth: state.currentMonth },
    actions,
  )
  useDiscoverRefresh(loadStats, loadMonthStats)

  const hasData = useMemo(() => {
    return state.weekStats.some((s) => s.word_count > 0) || state.monthStats.some((s) => s.word_count > 0)
  }, [state.monthStats, state.weekStats])

  const handleMonthChange = (year: number, month: number) => {
    actions.setCurrentYear(year)
    actions.setCurrentMonth(month)
  }

  return {
    loading: state.loading,
    hasData,
    weekStats: state.weekStats,
    monthStats: state.monthStats,
    yearStats: state.yearStats,
    consecutiveDays: state.consecutiveDays,
    maxConsecutiveDays: state.maxConsecutiveDays,
    currentYear: state.currentYear,
    currentMonth: state.currentMonth,
    handleMonthChange,
    handleYearChange,
  }
}

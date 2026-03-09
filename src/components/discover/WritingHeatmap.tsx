import { useMemo } from 'react'
import type { MonthSummary } from '@/features/discover-analytics'
import { useTranslation } from '@/hooks/use-translation'
import { buildWeeklyGrid } from './writing-heatmap-data'
import { WritingHeatmapView } from './writing-heatmap-view'

interface WritingHeatmapProps {
  yearStats: MonthSummary[]
  currentYear: number
  onYearChange: (year: number) => void
}

export function WritingHeatmap({ yearStats, currentYear, onYearChange }: WritingHeatmapProps) {
  const { language, translations } = useTranslation()

  const weeklyGrid = useMemo(() => {
    return buildWeeklyGrid(yearStats, currentYear)
  }, [yearStats, currentYear])

  const availableYears = useMemo(() => {
    const yearSet = new Set<number>()
    yearStats.forEach((month) => {
      yearSet.add(month.year)
    })
    yearSet.add(currentYear)
    return Array.from(yearSet).sort((a, b) => b - a)
  }, [yearStats, currentYear])

  return (
    <WritingHeatmapView
      translations={translations}
      language={language}
      currentYear={currentYear}
      availableYears={availableYears}
      weeklyGrid={weeklyGrid}
      onYearChange={onYearChange}
    />
  )
}

import { Spinner, TooltipProvider } from '@/magic-ui/components'
import { useSettingsStore } from '@/state/settings'
import { useProjectStore } from '@/state/project'
import { useDiscoverStats } from '@/hooks/use-discover-stats'
import { useHomePageStats } from '@/components/home/page/use-home-page-stats'
import { StatBoxRow } from './StatBoxRow'
import { ChartCard } from './ChartCard'
import { CalendarCard } from './CalendarCard'
import { HeatmapCard } from './HeatmapCard'
import { DiscoverEmptyState } from './discover-page-empty'

export function DiscoverPage() {
  const { projectsRootDir, dailyWordGoal } = useSettingsStore()
  const projectList = useProjectStore((state) => state.projectList)
  const projectCount = projectList.length
  const { totalWordCount } = useHomePageStats(projectList)

  const {
    loading,
    hasData,
    weekStats,
    monthStats,
    yearStats,
    currentYear,
    currentMonth,
    handleMonthChange,
    handleYearChange,
  } = useDiscoverStats(projectsRootDir)

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Spinner />
      </div>
    )
  }

  if (!hasData) {
    return <DiscoverEmptyState />
  }

  return (
    <TooltipProvider>
      <div className="bento-grid discover-dashboard">
        <StatBoxRow
          totalWords={totalWordCount}
          dailyGoal={dailyWordGoal}
          projectCount={projectCount}
        />

        <ChartCard stats={weekStats} />

        <CalendarCard
          monthStats={monthStats}
          currentYear={currentYear}
          currentMonth={currentMonth}
          onMonthChange={handleMonthChange}
        />

        <HeatmapCard
          yearStats={yearStats}
          currentYear={currentYear}
          onYearChange={handleYearChange}
        />
      </div>
    </TooltipProvider>
  )
}

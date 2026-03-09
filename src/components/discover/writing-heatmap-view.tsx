import { Activity } from 'lucide-react'
import type { Translations } from '@/i18n/locales/zh'
import type { DayCell } from './writing-heatmap-data'
import { HeatmapGrid } from './writing-heatmap-view-grid'

export function WritingHeatmapView(input: {
  translations: Translations
  language: string
  currentYear: number
  availableYears: number[]
  weeklyGrid: DayCell[][]
  onYearChange: (year: number) => void
}) {
  return (
    <>
      <div className="discover-heatmap-header-row">
        <h3 className="card-title">
          <Activity size={18} style={{ color: 'var(--primary)' }} />
          {input.translations.discover.heatmapTitle}
        </h3>

        <div className="discover-year-switch" aria-label="year-list">
          {input.availableYears.map((year) => (
            <button
              key={year}
              type="button"
              className={`year-btn ${year === input.currentYear ? 'year-btn-active' : ''}`}
              onClick={() => input.onYearChange(year)}
            >
              {year}
            </button>
          ))}
        </div>
      </div>

      <HeatmapGrid
        translations={input.translations}
        language={input.language}
        weeklyGrid={input.weeklyGrid}
      />
    </>
  )
}

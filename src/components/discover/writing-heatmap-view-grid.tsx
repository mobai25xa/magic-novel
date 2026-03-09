import { HEAT_COLORS, getHeatColor } from './writing-heatmap-config'
import type { DayCell } from './writing-heatmap-data'
import type { Translations } from '@/i18n/locales/zh'

export function HeatmapGrid(input: {
  translations: Translations
  language: string
  weeklyGrid: DayCell[][]
}) {
  const disc = input.translations.discover

  return (
    <>
      <div className="heatmap-scroll">
        <div className="heatmap-container">
          {input.weeklyGrid.map((week, weekIndex) => (
            <div key={weekIndex} className="heatmap-col">
              {week.map((day) => {
                const dateStr = day.date.toLocaleDateString(input.language === 'zh' ? 'zh-CN' : 'en-US')
                const levelClass = getHeatColor(day.wordCount)
                return (
                  <div
                    key={`${day.date.toISOString()}-${weekIndex}`}
                    className={`heatmap-cell ${levelClass}`}
                    title={`${dateStr}: ${day.wordCount} ${disc.wordUnit}`}
                  />
                )
              })}
            </div>
          ))}
        </div>
      </div>

      <div className="heatmap-legend">
        <span>{disc.less}</span>
        <div className="legend-box" aria-hidden="true">
          <div className="legend-cell" />
          <div className={`legend-cell ${HEAT_COLORS.low}`} />
          <div className={`legend-cell ${HEAT_COLORS.medium}`} />
          <div className={`legend-cell ${HEAT_COLORS.high}`} />
          <div className={`legend-cell ${HEAT_COLORS.peak}`} />
        </div>
        <span>{disc.more}</span>
      </div>
    </>
  )
}

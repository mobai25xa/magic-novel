import type { Translations } from '@/i18n/locales/zh'

export function HeatmapHeader(input: {
  translations: Translations
  selectedYear: number | 'recent'
  currentYear: number
  onSelectYear: (year: number | 'recent') => void
}) {
  const disc = input.translations.discover
  return (
    <div className="flex items-center justify-between mb-2">
      <div className="flex items-center gap-2">
        <h2 className="text-lg font-semibold">{disc.heatmapTitle}</h2>
        <button className="text-muted-foreground" style={{ transition: 'color 0.12s' }}>
          <svg className="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" strokeWidth="2" />
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 16v-4m0-4h.01" />
          </svg>
        </button>
      </div>

      <div className="flex gap-2">
        {[input.currentYear - 1, input.currentYear].map((year) => (
          <button
            key={year}
            onClick={() => input.onSelectYear(year)}
            className={`year-btn ${input.selectedYear === year ? 'year-btn-active' : ''}`}
          >
            {year}
          </button>
        ))}
        <button
          onClick={() => input.onSelectYear('recent')}
          className={`year-btn ${input.selectedYear === 'recent' ? 'year-btn-active' : ''}`}
        >
          {disc.recentYear}
        </button>
      </div>
    </div>
  )
}

export function HeatmapConsecutiveDaysText(input: { translations: Translations; consecutiveDays: number }) {
  if (input.consecutiveDays <= 0) {
    return null
  }

  const disc = input.translations.discover
  const parts = disc.consecutiveDays.split('{days}')

  return (
    <div className="mb-4 text-sm text-muted-foreground">
      {parts[0]}
      <span className="text-info-accent font-medium">{input.consecutiveDays}</span>
      {parts[1]}
    </div>
  )
}

export function HeatmapMaxStreakText(input: { translations: Translations; maxConsecutiveDays: number }) {
  const disc = input.translations.discover
  const parts = disc.maxConsecutiveDays.split('{days}')

  return (
    <div className="pt-4" style={{ borderTop: "1px solid var(--border-color)" }}>
      <div className="text-sm text-muted-foreground">
        {parts[0]}
        <span className="text-info-accent font-medium">{input.maxConsecutiveDays}</span>
        {parts[1]}
      </div>
    </div>
  )
}
import type { DailyStats } from '@/features/discover-analytics'
import type { Translations } from '@/i18n/locales/zh'

export interface ChartPoint {
  date: string
  label: string
  value: number
}

export function buildChartData(stats: DailyStats[], translations: Translations): ChartPoint[] {
  return stats.map((day, index) => {
    const value = day.word_count
    const date = new Date(day.date)
    const isToday = index === stats.length - 1
    const isYesterday = index === stats.length - 2

    let label = `${(date.getMonth() + 1).toString().padStart(2, '0')}/${date
      .getDate()
      .toString()
      .padStart(2, '0')}`

    if (isToday) label = translations.discover.today
    if (isYesterday) label = translations.discover.yesterday

    return { date: day.date, label, value }
  })
}

export function calculateMaxValue(chartData: ChartPoint[]): number {
  const max = Math.max(...chartData.map((d) => d.value), 1)
  const magnitude = Math.pow(10, Math.floor(Math.log10(max)))
  return Math.ceil(max / magnitude) * magnitude
}

export function buildYAxisLabels(maxValue: number): number[] {
  const labels = []
  const step = maxValue / 4
  for (let i = 0; i <= 4; i += 1) {
    labels.push(Math.round(step * i))
  }
  return labels
}

export function getWordUnit(translations: Translations): string {
  return translations.discover.wordUnit
}

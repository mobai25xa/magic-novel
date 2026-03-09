export const HEAT_COLORS = {
  zero: '',
  low: 'level-1',
  medium: 'level-2',
  high: 'level-3',
  peak: 'level-4',
} as const

export function getHeatColor(wordCount: number): string {
  if (wordCount === 0) return HEAT_COLORS.zero
  if (wordCount < 100) return HEAT_COLORS.low
  if (wordCount < 500) return HEAT_COLORS.medium
  if (wordCount < 1000) return HEAT_COLORS.high
  return HEAT_COLORS.peak
}

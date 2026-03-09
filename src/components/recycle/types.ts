export type RecycleItemType = 'novel' | 'chapter' | 'volume'

export type CountdownSeverity = 'safe' | 'warn' | 'urgent'

export interface RecycleItem {
  id: string
  name: string
  type: RecycleItemType
  origin: string
  description: string
  deletedAt: string
  daysRemaining: number
  source: 'project' | 'workspace'
}

export function getSeverity(daysRemaining: number): CountdownSeverity {
  if (daysRemaining >= 15) return 'safe'
  if (daysRemaining >= 5) return 'warn'
  return 'urgent'
}

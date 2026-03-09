import type { ReactNode } from 'react'

/** Bento 卡片通用 Props */
export interface BentoCardProps {
  className?: string
  span?: 4 | 6 | 8 | 12
  row?: 2 | 3
  children?: ReactNode
}

/** 统计卡片 Props */
export interface StatCardProps {
  icon: ReactNode
  value: string | number
  label: string
  trend?: { direction: 'up' | 'down'; value: string }
  iconTone?: 'blue' | 'green'
}

/** AI 灵感卡片 Props */
export interface AiTipCardProps {
  tip?: string
  onRefresh?: () => void
}

/** 书架项目 */
export interface BookShelfItem {
  path: string
  name: string
  author: string
  coverImage?: string
  wordCount?: number
}

import { TrendingUp } from 'lucide-react'

import type { StatCardProps } from './types'

export function StatCard({ icon, value, label, trend, iconTone = 'blue' }: StatCardProps) {
  return (
    <div className="bento-card bento-card-stat">
      <div className="bento-stat-header">
        <span>{label}</span>
        <div className={`bento-stat-icon-wrap bento-stat-icon-${iconTone}`}>
          {icon}
        </div>
      </div>

      <div className="bento-stat-val">{value}</div>

      {trend ? (
        <div className="bento-stat-trend">
          <TrendingUp size={14} />
          {trend.value}
        </div>
      ) : null}
    </div>
  )
}

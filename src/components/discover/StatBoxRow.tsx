import { BookText, Flame, PenTool } from 'lucide-react'
import { useT } from '@/hooks/use-translation'

interface StatBoxRowProps {
  totalWords: number
  dailyGoal: number
  projectCount: number
}

interface StatItem {
  key: string
  label: string
  value: string
  icon: React.ReactNode
  iconClass: 'purple' | 'blue' | 'green'
}

export function StatBoxRow({ totalWords, dailyGoal, projectCount }: StatBoxRowProps) {
  const t = useT()

  const stats: StatItem[] = [
    {
      key: 'total',
      label: `${t('home.totalWords')} (${t('home.words')})`,
      value: totalWords.toLocaleString(),
      icon: <PenTool size={22} />,
      iconClass: 'purple',
    },
    {
      key: 'daily-goal',
      label: t('editor.dailyGoal'),
      value: dailyGoal.toLocaleString(),
      icon: <Flame size={22} />,
      iconClass: 'blue',
    },
    {
      key: 'projects',
      label: `${t('home.totalProjects')} (${t('home.projects')})`,
      value: projectCount.toLocaleString(),
      icon: <BookText size={22} />,
      iconClass: 'green',
    },
  ]

  return (
    <div className="card-stats">
      {stats.map((item) => (
        <div key={item.key} className="stat-box">
          <div className={`stat-icon ${item.iconClass}`}>{item.icon}</div>
          <div className="stat-info">
            <span className="stat-label">{item.label}</span>
            <span className="stat-value">{item.value}</span>
          </div>
        </div>
      ))}
    </div>
  )
}

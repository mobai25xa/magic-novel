import { Clock, FileText, TrendingUp } from 'lucide-react'

import { formatLocaleNumber } from '../home-utils'

type Input = {
  projectCount: number
  totalWordCount: number
  language: string
  recentDateLabel: string
  popularType: { type: string; count: number } | null
  typeFilter: string | null
  labels: {
    totalProjects: string
    projects: string
    totalWords: string
    words: string
    recentUpdate: string
    popularType: string
    noData: string
    filtered: string
  }
  onToggleTypeFilter: (type: string) => void
}

function PopularTypeTag(input: Pick<Input, 'popularType' | 'typeFilter' | 'labels'>) {
  if (!input.popularType) {
    return <span>{input.labels.noData}</span>
  }

  return (
    <>
      <span>
        {input.popularType.type} ({input.popularType.count})
      </span>
      {input.typeFilter === input.popularType.type ? (
        <span className="tag tag-warning text-xs">{input.labels.filtered}</span>
      ) : null}
    </>
  )
}

export function HomePageStats(input: Input) {
  return (
    <div className="grid grid-cols-4 gap-5 mb-8">
      <div className="stat-card stat-card-success">
        <div className="stat-card-icon-float stat-card-icon-success">
          <FileText className="h-5 w-5" />
        </div>
        <div className="stat-card-label">{input.labels.totalProjects}</div>
        <div className="stat-card-value">
          {input.projectCount} <span className="stat-card-unit">{input.labels.projects}</span>
        </div>
      </div>

      <div className="stat-card stat-card-info">
        <div className="stat-card-icon-float stat-card-icon-info">
          <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
          </svg>
        </div>
        <div className="stat-card-label">{input.labels.totalWords}</div>
        <div className="stat-card-value">
          {formatLocaleNumber(input.totalWordCount, input.language)} <span className="stat-card-unit">{input.labels.words}</span>
        </div>
      </div>

      <div className="stat-card stat-card-danger">
        <div className="stat-card-icon-float stat-card-icon-danger">
          <Clock className="h-5 w-5" />
        </div>
        <div className="stat-card-label">{input.labels.recentUpdate}</div>
        <div className="stat-card-value stat-card-value-md">{input.recentDateLabel}</div>
      </div>

      <div
        className="stat-card stat-card-warning cursor-pointer hover:opacity-90 transition-opacity"
        onClick={() => input.popularType && input.onToggleTypeFilter(input.popularType.type)}
      >
        <div className="stat-card-icon-float stat-card-icon-warning">
          <TrendingUp className="h-5 w-5" />
        </div>
        <div className="stat-card-label">{input.labels.popularType}</div>
        <div className="stat-card-value stat-card-value-md flex items-center gap-2">
          <PopularTypeTag popularType={input.popularType} typeFilter={input.typeFilter} labels={input.labels} />
        </div>
      </div>
    </div>
  )
}
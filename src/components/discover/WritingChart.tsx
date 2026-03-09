import { useMemo, useState, type CSSProperties, type MouseEvent } from 'react'
import { useTranslation } from '@/hooks/use-translation'
import type { DailyStats } from '@/features/discover-analytics'
import {
  buildChartData,
  buildYAxisLabels,
  calculateMaxValue,
  getWordUnit,
  type ChartPoint,
} from './writing-chart-data'

interface WritingChartProps {
  stats: DailyStats[]
}

interface SeriesPoint extends ChartPoint {
  index: number
  x: number
  y: number
}

export function WritingChart({ stats }: WritingChartProps) {
  const { translations } = useTranslation()
  const chartData = useMemo(() => buildChartData(stats, translations), [stats, translations])
  const maxValue = useMemo(() => calculateMaxValue(chartData), [chartData])
  const yAxisLabels = useMemo(() => buildYAxisLabels(maxValue), [maxValue])
  const unit = getWordUnit(translations)

  const seriesPoints = useMemo(() => buildSeriesPoints(chartData, maxValue), [chartData, maxValue])
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null)

  const activePoint = hoveredIndex !== null ? seriesPoints[hoveredIndex] ?? null : null

  const handleChartMouseMove = (event: MouseEvent<HTMLDivElement>) => {
    if (seriesPoints.length === 0) {
      return
    }

    const rect = event.currentTarget.getBoundingClientRect()
    if (rect.width <= 0) {
      return
    }

    const ratio = (event.clientX - rect.left) / rect.width
    const clamped = Math.min(Math.max(ratio, 0), 1)
    const nextIndex = Math.round(clamped * (seriesPoints.length - 1))
    setHoveredIndex(nextIndex)
  }

  return (
    <div className="chart-body">
      <YAxis labels={yAxisLabels} />
      <div className="chart-main">
        <div
          className="chart-series-wrap"
          onMouseMove={handleChartMouseMove}
          onMouseLeave={() => setHoveredIndex(null)}
        >
          <GridLines />
          <LineSeries seriesPoints={seriesPoints} activePoint={activePoint} />
          <ChartPoints seriesPoints={seriesPoints} activePoint={activePoint} />
          {activePoint ? <ChartTooltip point={activePoint} unit={unit} /> : null}
        </div>
        <XAxisLabels seriesPoints={seriesPoints} />
      </div>
    </div>
  )
}

function YAxis({ labels }: { labels: number[] }) {
  return (
    <div className="chart-y-axis">
      {[...labels].reverse().map((label, index) => (
        <span key={index}>{label.toLocaleString()}</span>
      ))}
    </div>
  )
}

function GridLines() {
  return (
    <div className="chart-grid-lines">
      {[0, 1, 2, 3, 4].map((index) => (
        <div key={index} className="chart-grid-line" />
      ))}
    </div>
  )
}

function LineSeries(input: {
  seriesPoints: SeriesPoint[]
  activePoint: SeriesPoint | null
}) {
  const polylinePoints = input.seriesPoints.map((point) => `${point.x},${point.y}`).join(' ')
  const areaPoints = input.seriesPoints.length > 0 ? `${polylinePoints} 100,100 0,100` : ''

  return (
    <svg className="chart-svg" viewBox="0 0 100 100" preserveAspectRatio="none">
      <defs>
        <linearGradient id="discoverChartArea" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="rgba(5, 150, 105, 0.40)" />
          <stop offset="100%" stopColor="rgba(5, 150, 105, 0.00)" />
        </linearGradient>
      </defs>

      {areaPoints ? <polygon points={areaPoints} fill="url(#discoverChartArea)" /> : null}

      {input.activePoint ? (
        <line
          className="chart-guide-line"
          x1={input.activePoint.x}
          x2={input.activePoint.x}
          y1={0}
          y2={100}
          vectorEffect="non-scaling-stroke"
        />
      ) : null}

      <polyline
        fill="none"
        className="chart-line"
        vectorEffect="non-scaling-stroke"
        points={polylinePoints}
      />
    </svg>
  )
}

function ChartPoints(input: { seriesPoints: SeriesPoint[]; activePoint: SeriesPoint | null }) {
  return (
    <div className="chart-points-layer" aria-hidden="true">
      {input.seriesPoints.map((point) => {
        const isActive = input.activePoint?.index === point.index
        const style: CSSProperties = {
          left: `${point.x}%`,
          top: `${point.y}%`,
        }

        return (
          <span
            key={point.date}
            className={`chart-point-dot${isActive ? ' active' : ''}`}
            style={style}
          />
        )
      })}
    </div>
  )
}

function ChartTooltip(input: { point: SeriesPoint; unit: string }) {
  const style: CSSProperties = {
    left: `${input.point.x}%`,
    top: `${input.point.y}%`,
  }

  const alignClass = input.point.x < 10 ? 'edge-left' : input.point.x > 90 ? 'edge-right' : ''
  const valueText = input.unit
    ? `${input.point.value.toLocaleString()} ${input.unit}`
    : input.point.value.toLocaleString()

  return (
    <div className={`chart-tooltip ${alignClass}`.trim()} style={style}>
      <div className="chart-tooltip-label">{input.point.label}</div>
      <div className="chart-tooltip-value">{valueText}</div>
    </div>
  )
}

function XAxisLabels({ seriesPoints }: { seriesPoints: SeriesPoint[] }) {
  const lastIndex = seriesPoints.length - 1

  return (
    <div className="chart-x-axis">
      {seriesPoints.map((point, index) => {
        const classNames = ['chart-x-label']
        if (lastIndex === 0) {
          classNames.push('single')
        } else if (index === 0) {
          classNames.push('first')
        } else if (index === lastIndex) {
          classNames.push('last')
        }

        return (
          <span key={point.date} className={classNames.join(' ')} style={{ left: `${point.x}%` }}>
            {point.label}
          </span>
        )
      })}
    </div>
  )
}

function buildSeriesPoints(chartData: ChartPoint[], maxValue: number): SeriesPoint[] {
  const denominator = Math.max(chartData.length - 1, 1)

  return chartData.map((point, index) => {
    const x = (index / denominator) * 100
    const y = 100 - (point.value / maxValue) * 100
    return {
      ...point,
      index,
      x,
      y,
    }
  })
}

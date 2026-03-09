import { ChevronLeft, ChevronRight, Calendar as CalendarIcon } from 'lucide-react'
import type { DailyStats } from '@/features/discover-analytics'
import type { Translations } from '@/i18n/locales/zh'
import { formatDayWordCount, isToday } from './writing-calendar-data'

export function WritingCalendarView(input: {
  translations: Translations
  weekDays: string[]
  monthNames: string[]
  currentYear: number
  currentMonth: number
  calendarDays: Array<DailyStats | null>
  monthSummary: { writingDays: number; totalWords: number }
  onPrevMonth: () => void
  onNextMonth: () => void
}) {
  const disc = input.translations.discover
  return (
    <>
      <h3 className="card-title">
        <CalendarIcon size={18} style={{ color: 'var(--primary)' }} />
        {disc.calendarTitle}
      </h3>

      <div className="calendar-header">
        <span className="calendar-month">
          {input.currentYear}
          {disc.yearUnit}
          {input.monthNames[input.currentMonth - 1]}
        </span>
        <div className="calendar-actions">
          <button onClick={input.onPrevMonth} className="icon-btn" type="button" aria-label="prev-month">
            <ChevronLeft size={16} />
          </button>
          <button onClick={input.onNextMonth} className="icon-btn" type="button" aria-label="next-month">
            <ChevronRight size={16} />
          </button>
        </div>
      </div>

      <CalendarGrid weekDays={input.weekDays} calendarDays={input.calendarDays} />

      <MonthSummaryText
        template={disc.monthWritingSummary}
        writingDays={input.monthSummary.writingDays}
        totalWords={input.monthSummary.totalWords}
      />
    </>
  )
}

function CalendarGrid(input: {
  weekDays: string[]
  calendarDays: Array<DailyStats | null>
}) {
  return (
    <div className="calendar-grid">
      {input.weekDays.map((day) => (
        <div key={day} className="weekday">
          {day}
        </div>
      ))}

      {input.calendarDays.map((day, index) => {
        if (!day) {
          return <div key={`empty-${index}`} className="day empty" />
        }

        const dateNum = Number.parseInt(day.date.split('-')[2], 10)
        const today = isToday(day.date)
        const hasData = day.word_count > 0
        const wordLabel = formatDayWordCount(day.word_count)

        return (
          <div
            key={day.date}
            className={`day${today ? ' today' : ''}${hasData ? ' has-data' : ''}`}
            title={hasData ? `${day.word_count}` : undefined}
          >
            <span>{dateNum}</span>
            {wordLabel ? <span className="day-word-count">{wordLabel}</span> : null}
          </div>
        )
      })}
    </div>
  )
}

function MonthSummaryText(input: { template: string; writingDays: number; totalWords: number }) {
  return (
    <div className="discover-summary">
      {input.template
        .replace('{days}', String(input.writingDays))
        .replace('{words}', input.totalWords.toLocaleString())}
    </div>
  )
}

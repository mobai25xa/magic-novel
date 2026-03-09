import { format, isToday, isYesterday } from 'date-fns'

import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'

type MessageTimestampProps = {
  timestamp: number
  className?: string
}

function formatTimestamp(ts: number, yesterdayLabel: string) {
  const date = new Date(ts)

  if (isToday(date)) {
    return format(date, 'HH:mm')
  }

  if (isYesterday(date)) {
    return `${yesterdayLabel} ${format(date, 'HH:mm')}`
  }

  return format(date, 'MM/dd HH:mm')
}

export function MessageTimestamp({ timestamp, className }: MessageTimestampProps) {
  const ai = useAiTranslations()
  const label = formatTimestamp(timestamp, ai.timestamp.yesterday)

  return (
    <time
      dateTime={new Date(timestamp).toISOString()}
      className={cn('text-[11px] text-muted-foreground select-none', className)}
    >
      {label}
    </time>
  )
}

import { useEffect, useRef } from 'react'
import { Terminal } from 'lucide-react'
import { useTranslation } from '@/hooks/use-translation'
import type { LogEntry } from './types'

const DEMO_LOGS: LogEntry[] = [
  { time: '10:42:01', message: '[World Builder] 已连接至 Memory DB.', level: 'info' },
  { time: '10:43:15', message: '[Plot Generator] 生成了 3 种第 14 章的可能走向。', level: 'success' },
  { time: '10:45:00', message: '[Editor] 警告：主角性格在当前对话中出现轻微偏移。', level: 'warn' },
  { time: '10:45:32', message: '[Web Search] 正在检索"雨夜催债"的真实案例...', level: 'info' },
  { time: '10:46:11', message: 'Waiting for next event...', level: 'success' },
]

interface TerminalCardProps {
  logs?: LogEntry[]
}

export function TerminalCard({ logs = DEMO_LOGS }: TerminalCardProps) {
  const logEndRef = useRef<HTMLDivElement>(null)
  const { translations } = useTranslation()
  const wp = translations.workersPage

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [logs])

  return (
    <div className="bento-card card-terminal span-4">
      <div className="terminal-header">
        <div className="terminal-title">
          <Terminal size={16} />
          {wp.systemLogs}
        </div>
        <div className="terminal-status-group">
          <div className="terminal-status-dot" />
          {wp.active}
        </div>
      </div>
      <div className="terminal-log" role="log" aria-label="System logs">
        {logs.map((entry, i) => (
          <div className="log-item" key={i}>
            <span className="log-time">[{entry.time}]</span>
            <span className={`log-msg ${entry.level}`}>{entry.message}</span>
          </div>
        ))}
        <div ref={logEndRef} />
      </div>
    </div>
  )
}

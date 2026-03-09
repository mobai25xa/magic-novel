import type { LucideIcon } from 'lucide-react'

export type WorkerStatus = 'idle' | 'running' | 'standby'

export interface WorkerTool {
  name: string
  icon: LucideIcon
}

export interface WorkerCardData {
  id: string
  title: string
  subtitle: string
  icon: LucideIcon
  colorClass: 'bg-plot' | 'bg-world' | 'bg-editor' | 'bg-char'
  status: WorkerStatus
  statusLabel: string
  systemPrompt: string
  tools: WorkerTool[]
  primaryAction: string
  primaryActionIcon: LucideIcon
}

export interface LogEntry {
  time: string
  message: string
  level: 'info' | 'warn' | 'success'
}

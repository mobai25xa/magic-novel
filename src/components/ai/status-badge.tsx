import type { BadgeProps } from '@/magic-ui/components'
import { Badge } from '@/magic-ui/components'
import { cn } from '@/lib/utils'

type AiStatusBadgeProps = {
  status: string
  label?: string
  variant?: BadgeProps['variant']
  size?: BadgeProps['size']
  className?: string
}

function normalizeStatus(status: string) {
  return String(status ?? '').trim().toLowerCase()
}

function resolveStatusColor(status: string): BadgeProps['color'] {
  switch (normalizeStatus(status)) {
    case 'running':
    case 'in_progress':
    case 'initializing':
    case 'orchestrator_turn':
      return 'info'
    case 'paused':
    case 'waiting_confirmation':
    case 'waiting_askuser':
      return 'warning'
    case 'completed':
    case 'success':
      return 'success'
    case 'failed':
    case 'error':
      return 'error'
    default:
      return 'default'
  }
}

function resolveStatusLabel(status: string) {
  return String(status ?? '').replaceAll('_', ' ').trim() || 'unknown'
}

function shouldStrike(status: string) {
  const normalized = normalizeStatus(status)
  return normalized === 'cancelled' || normalized === 'canceled'
}

function shouldDot(status: string) {
  const normalized = normalizeStatus(status)
  return normalized === 'running' || normalized === 'in_progress' || normalized === 'initializing'
}

export function AiStatusBadge({ status, label, variant = 'soft', size = 'sm', className }: AiStatusBadgeProps) {
  return (
    <Badge
      color={resolveStatusColor(status)}
      variant={variant}
      size={size}
      dot={shouldDot(status)}
      className={cn(shouldStrike(status) && 'line-through', className)}
    >
      {label ?? resolveStatusLabel(status)}
    </Badge>
  )
}

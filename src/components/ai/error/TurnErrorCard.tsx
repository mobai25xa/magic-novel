import { useState, useCallback } from 'react'
import { Copy, Settings, RotateCcw } from 'lucide-react'

import { Button, Collapse } from '@/magic-ui/components'
import { useAiTranslations } from '../ai-hooks'
import type { ErrorDescriptor } from './classify-error'
import {
  CATEGORY_ICONS,
  CATEGORY_COLORS,
  CATEGORY_BORDER_COLORS,
  CATEGORY_BG_COLORS,
} from './error-ui-config'

interface TurnErrorCardProps {
  descriptor: ErrorDescriptor
  onRetry?: () => void
  onOpenSettings?: () => void
}

export function TurnErrorCard({ descriptor, onRetry, onOpenSettings }: TurnErrorCardProps) {
  const t = useAiTranslations()
  const [detailsOpen, setDetailsOpen] = useState(false)
  const [copied, setCopied] = useState(false)

  const errorCopy = t.error
  const codeEntry = errorCopy?.codes?.[descriptor.code]
  const title = codeEntry?.title ?? errorCopy?.fallbackTitle ?? 'Error'
  const description = codeEntry?.description ?? errorCopy?.fallbackDescription ?? ''
  const suggestion = codeEntry?.suggestion ?? errorCopy?.fallbackSuggestion ?? ''

  const Icon = CATEGORY_ICONS[descriptor.category]
  const colorClass = CATEGORY_COLORS[descriptor.category]
  const borderClass = CATEGORY_BORDER_COLORS[descriptor.category]
  const bgClass = CATEGORY_BG_COLORS[descriptor.category]

  const showRetry = descriptor.retryable && onRetry
  const showSettings = (descriptor.category === 'auth' || descriptor.category === 'network') && onOpenSettings

  const handleCopy = useCallback(() => {
    const text = [
      `code: ${descriptor.code}`,
      descriptor.provider ? `provider: ${descriptor.provider}` : '',
      descriptor.httpStatus ? `http_status: ${descriptor.httpStatus}` : '',
      descriptor.diagnostic ? `diagnostic: ${descriptor.diagnostic}` : '',
    ].filter(Boolean).join('\n')
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }, [descriptor])

  return (
    <div className={`border-l-[3px] ${borderClass} ${bgClass} rounded-r-md px-3 py-2.5 space-y-1.5`}>
      {/* Header: icon + title + action buttons */}
      <div className="flex items-center gap-2">
        <Icon className={`size-4 shrink-0 ${colorClass}`} />
        <span className={`text-sm font-medium ${colorClass} flex-1`}>{title}</span>
        <div className="flex items-center gap-1">
          {showRetry && (
            <Button
              onClick={onRetry}
              variant="ghost"
              size="sm"
              className="h-auto inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded"
            >
              <RotateCcw className="size-3" />
              {t.action.retryTurn}
            </Button>
          )}
          {showSettings && (
            <Button
              onClick={onOpenSettings}
              variant="ghost"
              size="sm"
              className="h-auto inline-flex items-center gap-1 px-2 py-0.5 text-xs rounded"
            >
              <Settings className="size-3" />
            </Button>
          )}
        </div>
      </div>

      {/* Description + suggestion */}
      {description && (
        <p className="text-xs text-secondary-foreground pl-6">{description}</p>
      )}
      {suggestion && (
        <p className="text-xs text-secondary-foreground pl-6">→ {suggestion}</p>
      )}

      {/* Expandable diagnostic details */}
      {descriptor.diagnostic && (
        <div className="pl-6">
          <Collapse
            collapsed={!detailsOpen}
            onCollapsedChange={(next) => setDetailsOpen(!next)}
            maxHeight={220}
            label={{
              expand: errorCopy?.expandDetails ?? 'Show details',
              collapse: errorCopy?.collapseDetails ?? 'Hide details',
            }}
          >
            <div className="mt-1 space-y-0.5 text-[11px] font-mono text-muted-foreground">
              <div>code: {descriptor.code}</div>
              {descriptor.provider && <div>{errorCopy?.providerLabel ?? 'Provider'}: {descriptor.provider}</div>}
              {descriptor.httpStatus && <div>{errorCopy?.httpStatusLabel ?? 'HTTP Status'}: {descriptor.httpStatus}</div>}
              <div>{errorCopy?.diagnosticLabel ?? 'Diagnostic'}: {descriptor.diagnostic}</div>
              <button
                onClick={handleCopy}
                className="inline-flex items-center gap-1 mt-1 text-[11px] hover:text-secondary-foreground"
              >
                <Copy className="size-3" />
                {copied ? t.action.copied : t.action.copyError}
              </button>
            </div>
          </Collapse>
        </div>
      )}
    </div>
  )
}

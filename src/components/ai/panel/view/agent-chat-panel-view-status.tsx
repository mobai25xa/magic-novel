import { useMemo, useState } from 'react'

import type { AgentPanelError } from '../../agent-chat-panel-utils'
import {
  formatAgentErrorDetails,
  hasExpandableAgentErrorDetails,
  toAgentPanelError,
} from '../../agent-chat-panel-utils'
import {
  AiErrorBody,
  AiErrorHeaderButton,
  AiErrorShell,
  Collapse,
  CodeBlock,
} from '@/magic-ui/components'
import { useAiTranslations } from '../../ai-hooks'
import { CATEGORY_ICONS, CATEGORY_COLORS } from '../../error/error-ui-config'
import {
  type ErrorCategory,
  TURN_ERROR_CODE_CATEGORY_MAP,
} from '../../error/classify-error'

/** Try to infer an ErrorCategory from the panel error's code or faultDomain */
function inferCategory(error: AgentPanelError): ErrorCategory | null {
  const code = error.code
  if (code && TURN_ERROR_CODE_CATEGORY_MAP[code]) {
    return TURN_ERROR_CODE_CATEGORY_MAP[code]
  }
  const fd = error.faultDomain
  if (fd === 'auth') return 'auth'
  if (fd === 'network' || fd === 'io') return 'network'
  if (fd === 'jvm' || fd === 'vc' || fd === 'external') return 'server'
  return null
}

type AgentChatPanelViewStatusProps = {
  lastError: AgentPanelError | null
  sessionError: string | null
  wasSessionResumed: boolean
  sessionRuntimeState: 'ready' | 'running' | 'suspended_confirmation' | 'suspended_askuser' | 'completed' | 'failed' | 'cancelled' | 'degraded'
  sessionCanContinue: boolean
  sessionCanResume: boolean
  sessionReadonlyReason?: string
  sessionHydrationStatus?: 'memory_hit' | 'snapshot_loaded' | 'event_rebuilt' | 'readonly_fallback'
  sessionWarnings: string[]
}

function ErrorRow(input: {
  title: string
  error: AgentPanelError
}) {
  const [collapsed, setCollapsed] = useState(true)
  const expandable = hasExpandableAgentErrorDetails(input.error)
  const details = useMemo(() => formatAgentErrorDetails(input.error), [input.error])
  const category = inferCategory(input.error)
  const Icon = category ? CATEGORY_ICONS[category] : null
  const colorClass = category ? CATEGORY_COLORS[category] : 'text-ai-status-error'

  if (!expandable) {
    return (
      <div className="flex items-center gap-1.5 text-ai-status-error">
        {Icon && <Icon className={`size-3.5 shrink-0 ${colorClass}`} />}
        {input.title}: {input.error.summary}
      </div>
    )
  }

  return (
    <AiErrorShell>
      <AiErrorHeaderButton
        onClick={() => setCollapsed((value) => !value)}
        aria-expanded={!collapsed}
        aria-label={input.title}
      >
        <span className="flex items-center gap-1.5 text-ai-status-error text-xs">
          {Icon && <Icon className={`size-3.5 shrink-0 ${colorClass}`} />}
          {input.title}: {input.error.summary}
        </span>
      </AiErrorHeaderButton>
      <Collapse
        collapsed={collapsed}
        onCollapsedChange={(next) => setCollapsed(next)}
        maxHeight={220}
      >
        <AiErrorBody
          style={{ borderColor: 'color-mix(in srgb, var(--ai-status-error) 20%, transparent)' }}
        >
          <CodeBlock className="max-h-48 overflow-auto whitespace-pre-wrap break-words text-[11px] text-ai-status-error">
            {details}
          </CodeBlock>
        </AiErrorBody>
      </Collapse>
    </AiErrorShell>
  )
}

export function AgentChatPanelViewStatus(input: AgentChatPanelViewStatusProps) {
  const ai = useAiTranslations()
  const normalizedSessionError = toAgentPanelError(input.sessionError)

  const errors = [input.lastError, normalizedSessionError].filter(Boolean) as AgentPanelError[]

  const hasStatusBanner = input.wasSessionResumed || input.sessionWarnings.length > 0
  if (errors.length === 0 && !hasStatusBanner) {
    return null
  }

  const readonlyReasonText = (() => {
    const reason = input.sessionReadonlyReason || ''
    if (!reason) return ai.panel.sessionReadOnlyReasonMissingRuntime

    if (reason === 'historical_suspended_session_without_runtime_snapshot') {
      return ai.panel.sessionReadOnlyReasonLegacySession
    }

    if (reason === 'provider_credentials_unavailable_for_resume') {
      return ai.panel.sessionReadOnlyReasonConfigMissing
    }

    if (reason === 'runtime_state_unavailable') {
      return ai.panel.sessionReadOnlyReasonMissingRuntime
    }

    return ai.panel.sessionReadOnlyReasonMissingRuntime
  })()

  const statusText = (() => {
    if (input.sessionCanContinue) {
      return ai.panel.sessionHydrated
    }

    if (input.sessionCanResume && input.sessionRuntimeState === 'suspended_confirmation') {
      return ai.panel.sessionHydratedWaitingConfirmation
    }

    if (input.sessionCanResume && input.sessionRuntimeState === 'suspended_askuser') {
      return ai.panel.sessionHydratedWaitingAskUser
    }

    return ai.panel.sessionHydratedReadOnly
  })()

  const statusClass = input.sessionCanContinue
    ? 'text-ai-status-success'
    : input.sessionCanResume
      ? 'text-warning'
      : 'text-warning'

  return (
    <div className="px-3 py-2 text-xs border-b space-y-1">
      {input.wasSessionResumed ? (
        <div className={statusClass}>{statusText}</div>
      ) : null}

      {input.wasSessionResumed && !input.sessionCanContinue && !input.sessionCanResume ? (
        <div className="text-warning/90">{readonlyReasonText}</div>
      ) : null}

      {input.sessionHydrationStatus === 'readonly_fallback' ? (
        <div className="text-warning/90">{ai.panel.sessionHydrationReadonlyFallback}</div>
      ) : null}

      {input.sessionWarnings.map((warning, index) => (
        <div key={`${warning}_${index}`} className="text-warning/80">
          {warning.startsWith('E_') ? `${ai.panel.requestFailed}: ${warning}` : warning}
        </div>
      ))}

      {errors.map((item, index) => (
        <ErrorRow
          key={`${item.summary}_${index}`}
          title={ai.panel.requestFailed}
          error={item}
        />
      ))}
    </div>
  )
}

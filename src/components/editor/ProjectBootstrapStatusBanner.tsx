import { useState } from 'react'
import { CheckCircle2, CircleAlert, Loader2, X } from 'lucide-react'

import {
  DEFAULT_CREATE_PROJECT_TARGET_REF,
  resolveBootstrapPhaseTranslationKey,
  resolveBootstrapRecommendedActionTranslationKey,
  resolveBootstrapRecommendedTargetRef,
  resolveCreateProjectHeadlineTranslationKey,
  resolveCreateProjectResultKind,
} from '@/components/create/workflow-helpers'
import { openEditorTarget } from '@/features/editor-navigation/open-editor-target'
import { useTranslation } from '@/hooks/use-translation'
import { Badge, Button } from '@/magic-ui/components'
import { useProjectStore } from '@/state/project'

function isActivePhase(phase: string) {
  return phase === 'pending'
    || phase === 'assembling_prompt'
    || phase === 'llm_generating'
    || phase === 'writing_artifacts'
}

function resolveBannerTone(phase: string) {
  if (phase === 'failed') {
    return {
      badgeColor: 'error' as const,
      borderClassName: 'border-red-500/30 bg-red-500/10',
    }
  }

  if (phase === 'partially_generated') {
    return {
      badgeColor: 'warning' as const,
      borderClassName: 'border-amber-500/30 bg-amber-500/10',
    }
  }

  if (isActivePhase(phase)) {
    return {
      badgeColor: 'info' as const,
      borderClassName: 'border-sky-500/30 bg-sky-500/10',
    }
  }

  return {
    badgeColor: 'success' as const,
    borderClassName: 'border-emerald-500/30 bg-emerald-500/10',
  }
}

function BannerIcon(input: { phase: string }) {
  if (input.phase === 'failed' || input.phase === 'partially_generated') {
    return <CircleAlert size={18} className="shrink-0" />
  }

  if (isActivePhase(input.phase)) {
    return <Loader2 size={18} className="shrink-0 animate-spin" />
  }

  return <CheckCircle2 size={18} className="shrink-0" />
}

export function ProjectBootstrapStatusBanner() {
  const { translations } = useTranslation()
  const project = useProjectStore((state) => state.project)
  const projectPath = useProjectStore((state) => state.projectPath)
  const bootstrapStatus = useProjectStore((state) => state.bootstrapStatus)
  const bootstrapStatusProjectPath = useProjectStore((state) => state.bootstrapStatusProjectPath)
  const clearBootstrapStatus = useProjectStore((state) => state.clearBootstrapStatus)
  const [dismissedKey, setDismissedKey] = useState<string | null>(null)

  const status = projectPath && bootstrapStatusProjectPath === projectPath
    ? bootstrapStatus
    : null

  const bannerKey = status
    ? `${projectPath}:${status.creation_job_id}:${status.phase}:${status.updated_at ?? 0}`
    : null

  if (!status || !projectPath || dismissedKey === bannerKey) {
    return null
  }

  const cp = translations.createPage
  const phaseKey = resolveBootstrapPhaseTranslationKey(status.phase)
  const recommendedActionKey = status.recommended_next_action
    ? resolveBootstrapRecommendedActionTranslationKey(status.recommended_next_action)
    : undefined
  const headlineKey = resolveCreateProjectHeadlineTranslationKey(resolveCreateProjectResultKind({
    bootstrapStatus: status,
    bootstrapError: status.phase === 'failed' ? status.error_message ?? null : null,
    bootstrapUnsupported: false,
  }))
  const tone = resolveBannerTone(status.phase)
  const active = isActivePhase(status.phase)
  const description = active
    ? cp.progressSubtitle.replace('{name}', project?.name ?? '')
    : status.phase === 'failed'
      ? (status.error_message?.trim() || cp.resultBootstrapFailedDesc)
      : cp.resultProjectReady.replace('{name}', project?.name ?? '')
  const recommendedText = recommendedActionKey ? cp[recommendedActionKey] : cp.recommendedContinuePlanning
  const targetRef = resolveBootstrapRecommendedTargetRef(status.recommended_next_action)
    ?? DEFAULT_CREATE_PROJECT_TARGET_REF

  return (
    <div className={`mb-4 rounded-2xl border p-4 ${tone.borderClassName}`}>
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0 flex-1 space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <BannerIcon phase={status.phase} />
            <div className="text-sm font-semibold">{cp[headlineKey]}</div>
            <Badge color={tone.badgeColor}>
              {phaseKey ? cp[phaseKey] : status.phase}
            </Badge>
            {active ? (
              <Badge color="info" variant="outline">
                {Math.max(0, Math.min(100, status.progress))}%
              </Badge>
            ) : null}
          </div>

          <p className="text-sm opacity-80">{description}</p>

          <div className="flex flex-wrap items-center gap-3 text-xs opacity-75">
            <span>{cp.progressCompletedSteps}: {status.completed_steps.length}</span>
            <span>{cp.progressGeneratedArtifacts}: {status.generated_artifacts.length}</span>
            <span>{cp.progressFailedSteps}: {status.failed_steps.length}</span>
          </div>

          <div className="rounded-xl border border-white/10 px-3 py-2 text-sm">
            <div className="text-xs opacity-70">{cp.progressRecommendedNext}</div>
            <div className="mt-1 font-medium">{recommendedText}</div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                void openEditorTarget(targetRef, {
                  revealLeftTree: true,
                  switchLeftTab: true,
                })
              }}
            >
              {translations.common.open}
            </Button>
            {!active ? (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  if (bannerKey) {
                    setDismissedKey(bannerKey)
                  }
                  clearBootstrapStatus(projectPath)
                }}
              >
                {translations.common.close}
              </Button>
            ) : null}
          </div>
        </div>

        <Button
          variant="ghost"
          size="icon"
          aria-label={translations.common.close}
          onClick={() => {
            if (bannerKey) {
              setDismissedKey(bannerKey)
            }
            if (!active) {
              clearBootstrapStatus(projectPath)
            }
          }}
        >
          <X size={16} />
        </Button>
      </div>
    </div>
  )
}

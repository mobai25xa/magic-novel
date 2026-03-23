import { Badge, Button } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'
import type { CreateProjectFlowResult } from '@/components/home/page/home-page-project-actions-helpers'
import {
  resolveBootstrapPhaseTranslationKey,
  resolveBootstrapRecommendedActionTranslationKey,
  resolveCreateProjectHeadlineTranslationKey,
  resolveCreateProjectResultKind,
} from '@/components/create/workflow-helpers'

interface CreateProjectResultPanelProps {
  mode: 'page' | 'dialog'
  result: CreateProjectFlowResult
  onEnterProject: () => void
  onRetryBootstrap: () => void
  onCreateAnother: () => void
  onClose: () => void
}

function ArtifactSummary(input: { result: CreateProjectFlowResult }) {
  const items = input.result.bootstrapStatus?.generated_artifacts ?? []
  if (items.length === 0) {
    return null
  }

  return (
    <div className="space-y-2">
      {items.slice(0, 8).map((artifact) => (
        <div key={`${artifact.kind}:${artifact.path}`} className="flex items-center justify-between gap-3 rounded-xl border border-[var(--border-primary)] px-3 py-2 text-sm">
          <div className="min-w-0">
            <div className="font-medium">{artifact.kind}</div>
            <div className="truncate text-xs opacity-70">{artifact.path}</div>
          </div>
          <Badge color={artifact.status === 'failed' ? 'error' : 'success'}>{artifact.status}</Badge>
        </div>
      ))}
    </div>
  )
}

export function CreateProjectResultPanel(input: CreateProjectResultPanelProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage
  const resultKind = resolveCreateProjectResultKind(input.result)
  const headline = cp[resolveCreateProjectHeadlineTranslationKey(resultKind)]
  const recommendedNext = cp[
    resolveBootstrapRecommendedActionTranslationKey(input.result.bootstrapStatus?.recommended_next_action)
  ]
  const canRetry = !input.result.bootstrapUnsupported
    && (resultKind === 'failed' || resultKind === 'partially_generated' || Boolean(input.result.bootstrapError))
  const phaseKey = resolveBootstrapPhaseTranslationKey(input.result.bootstrapStatus?.phase)

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-xl font-semibold">{headline}</h2>
        <p className="text-sm opacity-70">{cp.resultProjectReady.replace('{name}', input.result.snapshot.project.name)}</p>
      </div>

      <div className="space-y-3 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="text-xs opacity-70">{cp.resultProjectPath}</div>
            <div className="break-all text-sm">{input.result.snapshot.path}</div>
          </div>
          {input.result.bootstrapStatus?.phase ? (
            <Badge color={resultKind === 'failed' ? 'error' : 'success'}>
              {phaseKey ? cp[phaseKey] : input.result.bootstrapStatus.phase}
            </Badge>
          ) : null}
        </div>

        <div className="rounded-xl border border-[var(--border-primary)] px-3 py-3">
          <div className="text-xs opacity-70">{cp.progressRecommendedNext}</div>
          <div className="mt-1 text-sm font-medium">{recommendedNext}</div>
        </div>

        {input.result.bootstrapError ? (
          <div className="rounded-xl border border-amber-500/30 bg-amber-500/10 px-3 py-3 text-sm text-amber-200">
            {input.result.bootstrapUnsupported ? cp.resultBootstrapUnavailableDesc : cp.resultBootstrapFailedDesc}
          </div>
        ) : null}
      </div>

      <div className="space-y-3 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="text-sm font-medium">{cp.progressGeneratedArtifacts}</div>
        <ArtifactSummary result={input.result} />
      </div>

      <div className="flex flex-wrap justify-end gap-3">
        <Button variant="outline" onClick={input.onCreateAnother}>
          {cp.createAnother}
        </Button>
        {canRetry ? (
          <Button variant="outline" onClick={input.onRetryBootstrap}>
            {cp.retryBootstrap}
          </Button>
        ) : null}
        <Button variant={input.mode === 'dialog' ? 'outline' : 'secondary'} onClick={input.onClose}>
          {input.mode === 'dialog' ? cp.closeDialog : cp.backToWorkspace}
        </Button>
        <Button onClick={input.onEnterProject}>{cp.enterProject}</Button>
      </div>
    </div>
  )
}

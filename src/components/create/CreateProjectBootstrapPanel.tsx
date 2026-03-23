import { Badge, Button } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'
import type { ProjectBootstrapStatus } from '@/features/project-home'
import {
  resolveBootstrapPhaseTranslationKey,
  resolveBootstrapRecommendedActionTranslationKey,
} from '@/components/create/workflow-helpers'

interface CreateProjectBootstrapPanelProps {
  projectName: string
  status: ProjectBootstrapStatus | null
  onCancel: () => void
}

function ArtifactList(input: { items: ProjectBootstrapStatus['generated_artifacts'] }) {
  if (input.items.length === 0) {
    return null
  }

  return (
    <div className="space-y-2">
      {input.items.slice(0, 6).map((artifact) => (
        <div key={`${artifact.kind}:${artifact.path}`} className="flex items-center justify-between rounded-xl border border-[var(--border-primary)] px-3 py-2 text-sm">
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

export function CreateProjectBootstrapPanel(input: CreateProjectBootstrapPanelProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage
  const progress = input.status?.progress ?? 0
  const phaseKey = resolveBootstrapPhaseTranslationKey(input.status?.phase)
  const recommendedActionKey = input.status?.recommended_next_action
    ? resolveBootstrapRecommendedActionTranslationKey(input.status.recommended_next_action)
    : undefined

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-xl font-semibold">{cp.progressTitle}</h2>
        <p className="text-sm opacity-70">{cp.progressSubtitle.replace('{name}', input.projectName)}</p>
      </div>

      <div className="space-y-3 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <div className="text-sm font-medium">{cp.progressCurrentPhase}</div>
            <div className="text-lg font-semibold">
              {phaseKey ? cp[phaseKey] : cp.phasePending}
            </div>
          </div>
          <Badge color="info">{Math.max(0, Math.min(100, progress))}%</Badge>
        </div>

        <div className="h-2 overflow-hidden rounded-full bg-[var(--bg-muted)]">
          <div
            className="h-full rounded-full bg-[var(--color-primary-dark)] transition-all"
            style={{ width: `${Math.max(8, Math.min(100, progress || 8))}%` }}
          />
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <div className="rounded-xl border border-[var(--border-primary)] px-3 py-2">
            <div className="text-xs opacity-70">{cp.progressCompletedSteps}</div>
            <div className="text-lg font-semibold">{input.status?.completed_steps.length ?? 0}</div>
          </div>
          <div className="rounded-xl border border-[var(--border-primary)] px-3 py-2">
            <div className="text-xs opacity-70">{cp.progressGeneratedArtifacts}</div>
            <div className="text-lg font-semibold">{input.status?.generated_artifacts.length ?? 0}</div>
          </div>
          <div className="rounded-xl border border-[var(--border-primary)] px-3 py-2">
            <div className="text-xs opacity-70">{cp.progressFailedSteps}</div>
            <div className="text-lg font-semibold">{input.status?.failed_steps.length ?? 0}</div>
          </div>
        </div>
      </div>

      <div className="space-y-3 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex items-center justify-between gap-3">
          <div className="text-sm font-medium">{cp.progressGeneratedArtifacts}</div>
          {recommendedActionKey ? (
            <Badge color="success">{cp[recommendedActionKey]}</Badge>
          ) : null}
        </div>
        <ArtifactList items={input.status?.generated_artifacts ?? []} />
      </div>

      <div className="flex justify-end">
        <Button variant="outline" onClick={input.onCancel}>
          {cp.progressBackgroundAction}
        </Button>
      </div>
    </div>
  )
}

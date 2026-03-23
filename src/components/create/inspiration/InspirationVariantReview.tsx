import { useMemo } from 'react'
import { ArrowLeft, Layers3, Sparkles } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'
import { Button, Input, Tag, Textarea } from '@/magic-ui/components'

import { parseDelimitedItems } from './inspiration-helpers'
import type { useInspirationWorkflow } from './use-inspiration-workflow'

type InspirationWorkspaceViewModel = ReturnType<typeof useInspirationWorkflow>

interface InspirationVariantReviewProps {
  data: InspirationWorkspaceViewModel
  onBack: () => void
  onContinue: () => void
  onRegenerate: () => void | Promise<void>
}

export function InspirationVariantReview(input: InspirationVariantReviewProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage

  const finalDraft = input.data.finalCreateHandoffDraft
  const canContinue = Boolean(finalDraft?.name.trim() && finalDraft.description.trim())

  const selectedVariantLabel = useMemo(() => {
    const matched = input.data.variants.find(
      (candidate) => candidate.variant.variant_id === input.data.selectedVariantId,
    )
    return matched?.variant.label
  }, [input.data.selectedVariantId, input.data.variants])

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3 rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.2em] opacity-60">
            <Layers3 size={14} />
            <span>{cp.inspirationVariantStageLabel}</span>
          </div>
          <div className="text-lg font-semibold">{cp.inspirationVariantsTitle}</div>
          <p className="max-w-2xl text-sm opacity-75">{cp.inspirationVariantsSubtitle}</p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button variant="outline" onClick={input.onBack}>
            <ArrowLeft size={14} />
            {cp.inspirationBackToChat}
          </Button>
          <Button
            variant="outline"
            onClick={() => {
              void input.onRegenerate()
            }}
            disabled={input.data.generatingVariants}
          >
            {input.data.generatingVariants ? cp.inspirationGeneratingVariants : cp.inspirationRegenerateVariants}
          </Button>
          <Button onClick={input.onContinue} disabled={!canContinue}>
            {cp.inspirationContinueToForm}
          </Button>
        </div>
      </div>

      {input.data.sharedStoryCore ? (
        <div className="rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] px-4 py-3 text-sm">
          <span className="font-medium">{cp.inspirationSharedStoryCore}：</span>
          <span className="opacity-80">{input.data.sharedStoryCore}</span>
        </div>
      ) : null}

      <div className="grid gap-4 xl:grid-cols-3">
        {input.data.variants.map((candidate) => {
          const selected = candidate.variant.variant_id === input.data.selectedVariantId

          return (
            <div
              key={candidate.variant.variant_id}
              className={`rounded-[28px] border p-4 ${
                selected
                  ? 'border-[var(--color-primary-dark)] bg-[var(--bg-panel)]'
                  : 'border-[var(--border-primary)] bg-[var(--bg-panel)]'
              }`}
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="text-sm font-semibold">{candidate.variant.label}</div>
                  <p className="mt-1 text-xs opacity-70">{candidate.variant.one_liner}</p>
                </div>
                {selected ? <Tag variant="success">{cp.inspirationSelectedVariant}</Tag> : null}
              </div>

              <div className="mt-4 text-xl font-semibold">{candidate.variant.title}</div>

              <div className="mt-4 space-y-3 text-sm">
                <Section label={cp.inspirationVariantShortSynopsis} value={candidate.variant.short_synopsis} />
                <Section label={cp.inspirationVariantSettingSummary} value={candidate.variant.setting_summary} />
                <Section label={cp.inspirationVariantProtagonistSummary} value={candidate.variant.protagonist_summary} />
              </div>

              <div className="mt-4 flex flex-wrap gap-2">
                {candidate.variant.tags.map((tag) => (
                  <Tag key={`${candidate.variant.variant_id}_${tag}`} variant="outline-info">{tag}</Tag>
                ))}
              </div>

              <div className="mt-4">
                <Button
                  className="w-full"
                  variant={selected ? 'secondary' : 'outline'}
                  onClick={() => input.data.selectVariant(candidate)}
                >
                  {selected ? cp.inspirationCurrentFinalDraft : cp.inspirationUseVariant}
                </Button>
              </div>
            </div>
          )
        })}
      </div>

      <div className="rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Sparkles size={16} />
              <span>{cp.inspirationFinalDraftTitle}</span>
            </div>
            <p className="mt-2 text-xs opacity-70">{cp.inspirationFinalDraftDescription}</p>
          </div>
          {selectedVariantLabel ? <Tag variant="outline">{selectedVariantLabel}</Tag> : null}
        </div>

        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">{cp.titleLabel}</label>
            <Input
              value={finalDraft?.name ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ name: event.target.value })}
              placeholder={cp.titlePlaceholder}
            />
          </div>

          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">{cp.descLabel}</label>
            <Textarea
              value={finalDraft?.description ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ description: event.target.value })}
              placeholder={cp.descPlaceholder}
              rows={6}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.genreLabel}</label>
            <Input
              value={finalDraft?.project_type.join(', ') ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ project_type: parseDelimitedItems(event.target.value) })}
              placeholder={cp.customGenrePlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.toneLabel}</label>
            <Input
              value={finalDraft?.tone.join(', ') ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ tone: parseDelimitedItems(event.target.value) })}
              placeholder={cp.tonePlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.audienceLabel}</label>
            <Input
              value={finalDraft?.audience ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ audience: event.target.value })}
              placeholder={cp.audiencePlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.protagonistSeedLabel}</label>
            <Textarea
              value={finalDraft?.protagonist_seed ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ protagonist_seed: event.target.value })}
              placeholder={cp.protagonistSeedPlaceholder}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.counterpartSeedLabel}</label>
            <Textarea
              value={finalDraft?.counterpart_seed ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ counterpart_seed: event.target.value })}
              placeholder={cp.counterpartSeedPlaceholder}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.worldSeedLabel}</label>
            <Textarea
              value={finalDraft?.world_seed ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ world_seed: event.target.value })}
              placeholder={cp.worldSeedPlaceholder}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.endingDirectionLabel}</label>
            <Textarea
              value={finalDraft?.ending_direction ?? ''}
              onChange={(event) => input.data.updateFinalDraft({ ending_direction: event.target.value })}
              placeholder={cp.endingDirectionPlaceholder}
              rows={4}
            />
          </div>
        </div>
      </div>
    </div>
  )
}

function Section(input: { label: string; value: string }) {
  return (
    <div className="space-y-1">
      <div className="text-[11px] uppercase tracking-[0.18em] opacity-60">{input.label}</div>
      <div className="opacity-80">{input.value}</div>
    </div>
  )
}

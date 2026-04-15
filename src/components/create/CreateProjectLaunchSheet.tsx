import { FileText, Sparkles, User } from 'lucide-react'

import type { CreateProjectHandoffDraft } from '@/features/inspiration/types'
import { useTranslation } from '@/hooks/use-translation'
import { Button, Input, Tag, Textarea } from '@/magic-ui/components'

import type { CreateProjectDraft, CreateProjectFormErrors } from './types'

interface CreateProjectLaunchSheetProps {
  draft: CreateProjectDraft
  errors: CreateProjectFormErrors
  createHandoff: CreateProjectHandoffDraft | null
  submitting: boolean
  onChange: (patch: Partial<CreateProjectDraft>) => void
  onBack: () => void
  onSubmit: () => void
}

function ErrorText({ message }: { message?: string }) {
  if (!message) return null
  return <p className="text-xs text-red-500">{message}</p>
}

export function CreateProjectLaunchSheet(input: CreateProjectLaunchSheetProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage

  const contextTags = [
    ...(input.createHandoff?.project_type ?? []),
    ...(input.createHandoff?.tone ?? []),
  ]
  const audience = input.createHandoff?.audience?.trim()

  return (
    <div className="space-y-4">
      <div className="rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="space-y-2">
            <div className="flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.2em] opacity-60">
              <Sparkles size={14} />
              <span>{cp.launchSheetStageLabel}</span>
            </div>
            <div className="text-lg font-semibold">{cp.launchSheetTitle}</div>
            <p className="max-w-2xl text-sm opacity-75">{cp.launchSheetSubtitle}</p>
          </div>

          <Button variant="outline" onClick={input.onBack} disabled={input.submitting}>
            {cp.inspirationBackToWorkspaceFlow}
          </Button>
        </div>

        {contextTags.length > 0 || audience ? (
          <div className="mt-4 flex flex-wrap gap-2">
            {contextTags.map((item) => (
              <Tag key={item} variant="outline-info">{item}</Tag>
            ))}
            {audience ? <Tag variant="outline">{audience}</Tag> : null}
          </div>
        ) : (
          <p className="mt-4 text-sm opacity-60">{cp.launchSheetContextEmpty}</p>
        )}
      </div>

      <div className="rounded-[28px] border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">{cp.titleLabel}</label>
            <Input
              value={input.draft.name}
              onChange={(event) => input.onChange({ name: event.target.value })}
              placeholder={cp.titlePlaceholder}
            />
            <ErrorText message={input.errors.name ? cp.validationNameRequired : undefined} />
          </div>

          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">{cp.descLabel}</label>
            <Textarea
              value={input.draft.description}
              onChange={(event) => input.onChange({ description: event.target.value })}
              placeholder={cp.descPlaceholder}
              rows={8}
            />
            <ErrorText message={input.errors.description ? cp.validationDescriptionRequired : undefined} />
          </div>

          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">{cp.authorLabel}</label>
            <Input
              value={input.draft.author}
              onChange={(event) => input.onChange({ author: event.target.value })}
              placeholder={cp.authorPlaceholder}
            />
            <ErrorText message={input.errors.author ? cp.validationAuthorRequired : undefined} />
          </div>
        </div>

        <div className="mt-6 grid gap-3 md:grid-cols-3">
          <div className="rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] px-4 py-3">
            <div className="flex items-center gap-2 text-sm font-medium">
              <FileText size={15} />
              <span>{cp.launchSheetContractReadyTitle}</span>
            </div>
            <p className="mt-2 text-xs opacity-70">{cp.launchSheetContractReadyDescription}</p>
          </div>
          <div className="rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] px-4 py-3">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Sparkles size={15} />
              <span>{cp.launchSheetNoBootstrapTitle}</span>
            </div>
            <p className="mt-2 text-xs opacity-70">{cp.launchSheetNoBootstrapDescription}</p>
          </div>
          <div className="rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-base)] px-4 py-3">
            <div className="flex items-center gap-2 text-sm font-medium">
              <User size={15} />
              <span>{cp.launchSheetProjectHomeTitle}</span>
            </div>
            <p className="mt-2 text-xs opacity-70">{cp.launchSheetProjectHomeDescription}</p>
          </div>
        </div>

        <div className="mt-6 flex flex-wrap justify-end gap-3">
          <Button variant="outline" onClick={input.onBack} disabled={input.submitting}>
            {translations.common.back}
          </Button>
          <Button onClick={input.onSubmit} disabled={input.submitting}>
            {input.submitting ? cp.generatingContractCta : cp.submit}
          </Button>
        </div>
      </div>
    </div>
  )
}

import { useMemo, useRef } from 'react'
import type { ChangeEvent, ReactNode } from 'react'
import { BookOpen, ImagePlus, Layers3, Settings2, Users } from 'lucide-react'

import {
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Tag,
  Textarea,
} from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

import { AiAssistBox } from './AiAssistBox'
import type { CreateProjectDraft, CreateProjectFormErrors } from './types'

interface CreateProjectFormProps {
  draft: CreateProjectDraft
  errors: CreateProjectFormErrors
  mode: 'page' | 'dialog'
  projectGenres: string[]
  submitting: boolean
  onChange: (patch: Partial<CreateProjectDraft>) => void
  onToggleGenre: (genre: string) => void
  onCancel: () => void
  onSubmit: () => void
}

function SectionHeader(input: { icon: ReactNode; title: string; description: string }) {
  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2 text-sm font-semibold">
        {input.icon}
        <span>{input.title}</span>
      </div>
      <p className="text-xs opacity-70">{input.description}</p>
    </div>
  )
}

function ErrorText(input: { message?: string }) {
  if (!input.message) return null
  return <p className="text-xs text-red-500">{input.message}</p>
}

function resolveErrorMessage(
  key: string | undefined,
  messages: {
    name: string
    author: string
    description: string
    projectType: string
    targetTotalWords: string
  },
) {
  if (!key || !(key in messages)) return undefined
  return messages[key as keyof typeof messages]
}

function CoverPicker(input: {
  value: string
  pickLabel: string
  onChange: (coverImage: string) => void
}) {
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFileChange = (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file || !file.type.startsWith('image/')) {
      return
    }

    const reader = new FileReader()
    reader.onload = (readerEvent) => {
      const result = readerEvent.target?.result
      if (typeof result === 'string') {
        input.onChange(result)
      }
    }
    reader.readAsDataURL(file)
  }

  return (
    <div className="space-y-3">
      <input
        ref={fileInputRef}
        type="file"
        hidden
        accept="image/*"
        onChange={handleFileChange}
      />
      <button
        type="button"
        onClick={() => fileInputRef.current?.click()}
        className="flex h-32 w-28 items-center justify-center overflow-hidden rounded-xl border border-dashed border-[var(--border-primary)] bg-[var(--bg-panel)]"
      >
        {input.value ? (
          <img src={input.value} alt={input.pickLabel} className="h-full w-full object-cover" />
        ) : (
          <div className="space-y-2 text-center text-xs opacity-70">
            <ImagePlus size={24} className="mx-auto" />
            <span>{input.pickLabel}</span>
          </div>
        )}
      </button>
    </div>
  )
}

export function CreateProjectForm(input: CreateProjectFormProps) {
  const { translations } = useTranslation()
  const cp = translations.createPage

  const errorMessages = useMemo(
    () => ({
      name: cp.validationNameRequired,
      author: cp.validationAuthorRequired,
      description: cp.validationDescriptionRequired,
      projectType: cp.validationGenreRequired,
      targetTotalWords: cp.validationTargetWordsRequired,
    }),
    [cp],
  )

  return (
    <div className="space-y-6">
      <div className={`grid gap-6 ${input.mode === 'dialog' ? 'lg:grid-cols-[128px_minmax(0,1fr)]' : 'lg:grid-cols-[144px_minmax(0,1fr)]'}`}>
        <CoverPicker
          value={input.draft.coverImage}
          pickLabel={cp.coverPicker}
          onChange={(coverImage) => input.onChange({ coverImage })}
        />

        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2 md:col-span-2">
            <label className="text-sm font-medium">{cp.titleLabel}</label>
            <Input
              value={input.draft.name}
              onChange={(event) => input.onChange({ name: event.target.value })}
              placeholder={cp.titlePlaceholder}
            />
            <ErrorText message={resolveErrorMessage(input.errors.name, errorMessages)} />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.authorLabel}</label>
            <Input
              value={input.draft.author}
              onChange={(event) => input.onChange({ author: event.target.value })}
              placeholder={cp.authorPlaceholder}
            />
            <ErrorText message={resolveErrorMessage(input.errors.author, errorMessages)} />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.targetWordsLabel}</label>
            <Input
              type="number"
              min={1}
              value={input.draft.targetTotalWords}
              onChange={(event) => input.onChange({ targetTotalWords: event.target.value })}
              placeholder={cp.targetWordsPlaceholder}
            />
            <ErrorText message={resolveErrorMessage(input.errors.targetTotalWords, errorMessages)} />
          </div>
        </div>
      </div>

      <div className="space-y-4 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <SectionHeader
          icon={<BookOpen size={16} />}
          title={cp.requiredSectionTitle}
          description={cp.requiredSectionDescription}
        />

        <div className="space-y-2">
          <label className="text-sm font-medium">{cp.genreLabel}</label>
          <div className="flex flex-wrap gap-2">
            {input.projectGenres.map((genre) => (
              <button key={genre} type="button" onClick={() => input.onToggleGenre(genre)} className="rounded">
                <Tag variant={input.draft.selectedGenres.includes(genre) ? 'success' : 'outline'}>
                  {genre}
                </Tag>
              </button>
            ))}
          </div>
          <Input
            value={input.draft.customGenres}
            onChange={(event) => input.onChange({ customGenres: event.target.value })}
            placeholder={cp.customGenrePlaceholder}
          />
          <ErrorText message={resolveErrorMessage(input.errors.projectType, errorMessages)} />
        </div>

        <div className="space-y-2">
          <label className="text-sm font-medium">{cp.descLabel}</label>
          <Textarea
            value={input.draft.description}
            onChange={(event) => input.onChange({ description: event.target.value })}
            placeholder={cp.descPlaceholder}
            rows={5}
          />
          <ErrorText message={resolveErrorMessage(input.errors.description, errorMessages)} />
        </div>
      </div>

      <div className="space-y-4 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <SectionHeader
          icon={<Layers3 size={16} />}
          title={cp.suggestedSectionTitle}
          description={cp.suggestedSectionDescription}
        />

        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.plannedVolumesLabel}</label>
            <Input
              type="number"
              min={1}
              value={input.draft.plannedVolumes}
              onChange={(event) => input.onChange({ plannedVolumes: event.target.value })}
              placeholder={cp.plannedVolumesPlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.targetWordsPerChapterLabel}</label>
            <Input
              type="number"
              min={1}
              value={input.draft.targetWordsPerChapter}
              onChange={(event) => input.onChange({ targetWordsPerChapter: event.target.value })}
              placeholder={cp.targetWordsPerChapterPlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.protagonistSeedLabel}</label>
            <Textarea
              value={input.draft.protagonistSeed}
              onChange={(event) => input.onChange({ protagonistSeed: event.target.value })}
              placeholder={cp.protagonistSeedPlaceholder}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.counterpartSeedLabel}</label>
            <Textarea
              value={input.draft.counterpartSeed}
              onChange={(event) => input.onChange({ counterpartSeed: event.target.value })}
              placeholder={cp.counterpartSeedPlaceholder}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.worldSeedLabel}</label>
            <Textarea
              value={input.draft.worldSeed}
              onChange={(event) => input.onChange({ worldSeed: event.target.value })}
              placeholder={cp.worldSeedPlaceholder}
              rows={4}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.endingDirectionLabel}</label>
            <Textarea
              value={input.draft.endingDirection}
              onChange={(event) => input.onChange({ endingDirection: event.target.value })}
              placeholder={cp.endingDirectionPlaceholder}
              rows={4}
            />
          </div>
        </div>
      </div>

      <div className="space-y-4 rounded-2xl border border-[var(--border-primary)] bg-[var(--bg-panel)] p-4">
        <SectionHeader
          icon={<Settings2 size={16} />}
          title={cp.advancedSectionTitle}
          description={cp.advancedSectionDescription}
        />

        <div className="grid gap-4 md:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.narrativePovLabel}</label>
            <Select
              value={input.draft.narrativePov}
              onValueChange={(value) => input.onChange({ narrativePov: value as CreateProjectDraft['narrativePov'] })}
            >
              <SelectTrigger className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="first_person">{cp.narrativePovFirstPerson}</SelectItem>
                <SelectItem value="third_limited">{cp.narrativePovThirdLimited}</SelectItem>
                <SelectItem value="third_omniscient">{cp.narrativePovThirdOmniscient}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.audienceLabel}</label>
            <Input
              value={input.draft.audience}
              onChange={(event) => input.onChange({ audience: event.target.value })}
              placeholder={cp.audiencePlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.toneLabel}</label>
            <Input
              value={input.draft.tone}
              onChange={(event) => input.onChange({ tone: event.target.value })}
              placeholder={cp.tonePlaceholder}
            />
          </div>

          <div className="space-y-2">
            <label className="text-sm font-medium">{cp.workflowModeLabel}</label>
            <div className="rounded-xl border border-[var(--border-primary)] px-3 py-2 text-sm opacity-80">
              <div className="mb-1 flex items-center gap-2 font-medium">
                <Users size={16} />
                {input.draft.aiAssist ? cp.workflowModeAi : cp.workflowModeManual}
              </div>
              <p className="text-xs opacity-70">
                {input.draft.aiAssist ? cp.workflowModeAiDescription : cp.workflowModeManualDescription}
              </p>
            </div>
          </div>
        </div>
      </div>

      <AiAssistBox enabled={input.draft.aiAssist} onToggle={() => input.onChange({ aiAssist: !input.draft.aiAssist })} />

      <div className="flex flex-wrap justify-end gap-3">
        <Button variant="outline" onClick={input.onCancel} disabled={input.submitting}>
          {input.mode === 'dialog' ? cp.closeDialog : cp.cancel}
        </Button>
        <Button onClick={input.onSubmit} disabled={input.submitting}>
          {input.submitting ? cp.submitting : cp.submit}
        </Button>
      </div>
    </div>
  )
}

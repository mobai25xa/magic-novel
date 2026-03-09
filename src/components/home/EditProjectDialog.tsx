import { useState, useRef } from 'react'
import { Input, Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle, Tag, Textarea } from '@/magic-ui/components'
import { useSettingsStore } from '@/stores/settings-store'
import { useTranslation } from '@/hooks/use-translation'
import { ProjectDialogButton } from './project-dialog-button'

type EditProjectPayload = {
  name: string
  author: string
  description?: string
  coverImage?: string
  projectType?: string[]
}

interface EditProjectDialogProps {
  open: boolean
  onClose: () => void
  onConfirm: (data: EditProjectPayload) => void
  initialData: {
    name: string
    author: string
    description?: string
    coverImage?: string
    projectType?: string[]
  }
}

function EditProjectCover({
  coverImage,
  onPick,
  labels,
}: {
  coverImage: string
  onPick: () => void
  labels: { cover: string; addCover: string }
}) {
  return (
    <div className="flex-shrink-0">
      <div
        onClick={onPick}
        className="cover-placeholder"
      >
        {coverImage ? (
          <img src={coverImage} alt={labels.cover} className="w-full h-full object-cover" />
        ) : (
          <div className="text-center text-sm">
            <svg className="w-12 h-12 mx-auto mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"
              />
            </svg>
            <div>{labels.addCover}</div>
          </div>
        )}
      </div>
    </div>
  )
}

function EditProjectFields(input: {
  name: string
  author: string
  description: string
  onNameChange: (value: string) => void
  onAuthorChange: (value: string) => void
  onDescriptionChange: (value: string) => void
  labels: {
    name: string
    namePlaceholder: string
    author: string
    authorPlaceholder: string
    description: string
    descriptionPlaceholder: string
  }
}) {
  return (
    <div className="flex-1 flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <label className="form-label w-16">{input.labels.name}</label>
        <Input
          value={input.name}
          onChange={(e) => input.onNameChange(e.target.value)}
          placeholder={input.labels.namePlaceholder}
          autoFocus
          className="flex-1"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="form-label w-16">{input.labels.author}</label>
        <Input
          value={input.author}
          onChange={(e) => input.onAuthorChange(e.target.value)}
          placeholder={input.labels.authorPlaceholder}
          className="flex-1"
        />
      </div>

      <div className="flex items-start gap-2">
        <label className="form-label w-16 pt-2">{input.labels.description}</label>
        <Textarea
          value={input.description}
          onChange={(e) => input.onDescriptionChange(e.target.value)}
          placeholder={input.labels.descriptionPlaceholder}
          className="flex-1 min-h-[80px]"
        />
      </div>
    </div>
  )
}

function EditProjectGenres(input: {
  title: string
  projectGenres: string[]
  genres: string[]
  onToggle: (genre: string) => void
  onRemove: (genre: string) => void
}) {
  return (
    <div className="mb-6">
      <label className="form-label font-medium mb-2 block">{input.title}</label>

      <div className="flex flex-wrap gap-2">
        {input.projectGenres.map((genre) => (
          <button key={genre} type="button" onClick={() => input.onToggle(genre)} className="rounded">
            <Tag variant={input.genres.includes(genre) ? 'success' : 'outline'}>{genre}</Tag>
          </button>
        ))}
      </div>

      {input.genres.length > 0 && (
        <div className="flex flex-wrap gap-2 mt-3">
          {input.genres.map((genre) => (
            <Tag key={genre} variant="success" closable onClose={() => input.onRemove(genre)}>
              {genre}
            </Tag>
          ))}
        </div>
      )}
    </div>
  )
}

function toggleGenreSelection(previous: string[], genre: string) {
  return previous.includes(genre)
    ? previous.filter((item) => item !== genre)
    : [...previous, genre]
}

function buildEditProjectPayload(input: {
  name: string
  author: string
  description: string
  coverImage: string
  genres: string[]
}): EditProjectPayload | null {
  const trimmedName = input.name.trim()
  const trimmedAuthor = input.author.trim()
  if (!trimmedName || !trimmedAuthor) return null

  return {
    name: trimmedName,
    author: trimmedAuthor,
    description: input.description.trim() || undefined,
    coverImage: input.coverImage || undefined,
    projectType: input.genres,
  }
}

function EditProjectActions(input: {
  name: string
  author: string
  description: string
  coverImage: string
  genres: string[]
  onClose: () => void
  onConfirm: (data: EditProjectPayload) => void
  labels: { cancel: string; confirm: string }
}) {
  const payload = buildEditProjectPayload({
    name: input.name,
    author: input.author,
    description: input.description,
    coverImage: input.coverImage,
    genres: input.genres,
  })

  return (
    <div className="dialog-actions">
      <ProjectDialogButton variant="outline" onClick={input.onClose}>
        {input.labels.cancel}
      </ProjectDialogButton>
      <ProjectDialogButton
        onClick={() => {
          if (!payload) return
          input.onConfirm(payload)
          input.onClose()
        }}
        disabled={!payload}
      >
        {input.labels.confirm}
      </ProjectDialogButton>
    </div>
  )
}

export function EditProjectDialog({ open, onClose, onConfirm, initialData }: EditProjectDialogProps) {
  const { translations } = useTranslation()
  const { projectGenres } = useSettingsStore()
  const pd = translations.projectDialog
  const common = translations.common

  const [name, setName] = useState(initialData.name)
  const [author, setAuthor] = useState(initialData.author)
  const [description, setDescription] = useState(initialData.description || '')
  const [coverImage, setCoverImage] = useState(initialData.coverImage || '')
  const [genres, setGenres] = useState<string[]>(initialData.projectType || [])
  const fileInputRef = useRef<HTMLInputElement>(null)

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file || !file.type.startsWith('image/')) {
      return
    }

    const reader = new FileReader()
    reader.onload = (event) => {
      const result = event.target?.result as string
      setCoverImage(result)
    }
    reader.readAsDataURL(file)
  }

  return (
    <Modal open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <ModalContent size="lg">
        <ModalHeader>
          <ModalTitle>{pd.editProject}</ModalTitle>
          <ModalDescription className="sr-only">{pd.editProject}</ModalDescription>
        </ModalHeader>
        <div className="p-6">
          <input
            ref={fileInputRef}
            type="file"
            accept="image/*"
            onChange={handleFileChange}
            hidden
            aria-hidden="true"
            tabIndex={-1}
          />
          <div className="flex gap-6 mb-6">
            <EditProjectCover
              coverImage={coverImage}
              onPick={() => fileInputRef.current?.click()}
              labels={{ cover: pd.cover, addCover: pd.addCover }}
            />
            <EditProjectFields
              name={name}
              author={author}
              description={description}
              onNameChange={setName}
              onAuthorChange={setAuthor}
              onDescriptionChange={setDescription}
              labels={{
                name: pd.bookName,
                namePlaceholder: pd.bookNamePlaceholder,
                author: pd.author,
                authorPlaceholder: pd.authorPlaceholder,
                description: pd.description,
                descriptionPlaceholder: pd.descriptionPlaceholder,
              }}
            />
          </div>

          <EditProjectGenres
            title={translations.projectType.type}
            projectGenres={projectGenres}
            genres={genres}
            onToggle={(genre) => setGenres((prev) => toggleGenreSelection(prev, genre))}
            onRemove={(genre) => setGenres((prev) => prev.filter((item) => item !== genre))}
          />

          <EditProjectActions
            name={name}
            author={author}
            description={description}
            coverImage={coverImage}
            genres={genres}
            onClose={onClose}
            onConfirm={onConfirm}
            labels={{ cancel: common.cancel, confirm: common.confirm }}
          />
        </div>
      </ModalContent>
    </Modal>
  )
}

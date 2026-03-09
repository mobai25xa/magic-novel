import { useRef, useState } from 'react'
import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from '@/magic-ui/components'
import { Input } from '@/magic-ui/components'
import { Tag } from '@/magic-ui/components'
import { useSettingsStore } from '@/stores/settings-store'
import { useTranslation } from '@/hooks/use-translation'
import { ProjectDialogButton } from './project-dialog-button'

interface CreateProjectDialogProps {
  open: boolean
  onClose: () => void
  onConfirm: (data: { name: string; author: string; tags: string; coverImage?: string; projectType: string[] }) => void
}

function CreateProjectCover({
  coverImage,
  onPick,
  label,
}: {
  coverImage: string
  onPick: () => void
  label: string
}) {
  return (
    <div className="flex-shrink-0">
      <div onClick={onPick} className="cover-placeholder">
        {coverImage ? (
          <img src={coverImage} alt={label} className="w-full h-full object-cover" />
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
            <div>{label}</div>
          </div>
        )}
      </div>
    </div>
  )
}

function CreateProjectBasicFields(input: {
  name: string
  author: string
  tags: string
  labels: { bookName: string; bookNamePlaceholder: string; author: string; authorPlaceholder: string; tags: string; tagsPlaceholder: string }
  onNameChange: (value: string) => void
  onAuthorChange: (value: string) => void
  onTagsChange: (value: string) => void
}) {
  const lb = input.labels
  return (
    <div className="flex-1 h-40 flex flex-col justify-between">
      <div className="flex items-center gap-2">
        <label className="form-label">{lb.bookName}</label>
        <Input
          value={input.name}
          onChange={(e) => input.onNameChange(e.target.value)}
          placeholder={lb.bookNamePlaceholder}
          autoFocus
          className="flex-1"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="form-label">{lb.author}</label>
        <Input
          value={input.author}
          onChange={(e) => input.onAuthorChange(e.target.value)}
          placeholder={lb.authorPlaceholder}
          className="flex-1"
        />
      </div>

      <div className="flex items-center gap-2">
        <label className="form-label">{lb.tags}</label>
        <Input
          value={input.tags}
          onChange={(e) => input.onTagsChange(e.target.value)}
          placeholder={lb.tagsPlaceholder}
          className="flex-1"
        />
      </div>
    </div>
  )
}

function ProjectGenreSelector(input: {
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

export function CreateProjectDialog({ open, onClose, onConfirm }: CreateProjectDialogProps) {
  const { translations } = useTranslation()
  const pd = translations.projectDialog
  const { projectGenres } = useSettingsStore()

  const [name, setName] = useState('')
  const [author, setAuthor] = useState('')
  const [tags, setTags] = useState(pd.defaultName)
  const [genres, setGenres] = useState<string[]>([])
  const [coverImage, setCoverImage] = useState('')
  const fileInputRef = useRef<HTMLInputElement>(null)

  const toggleGenre = (genre: string) => {
    setGenres((prev) => (prev.includes(genre) ? prev.filter((item) => item !== genre) : [...prev, genre]))
  }

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

  const handleClose = () => {
    setName('')
    setAuthor('')
    setTags(pd.defaultName)
    setGenres([])
    setCoverImage('')
    onClose()
  }

  const handleConfirm = () => {
    if (!name.trim() || !author.trim()) return
    onConfirm({
      name: name.trim(),
      author: author.trim(),
      tags: tags.trim(),
      coverImage: coverImage || undefined,
      projectType: genres,
    })
    handleClose()
  }

  return (
    <Modal open={open} onOpenChange={(isOpen) => !isOpen && handleClose()}>
      <ModalContent size="lg">
        <ModalHeader>
          <ModalTitle>{translations.home.createProject}</ModalTitle>
          <ModalDescription className="sr-only">{translations.home.createProject}</ModalDescription>
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
            <CreateProjectCover
              coverImage={coverImage}
              onPick={() => fileInputRef.current?.click()}
              label={pd.addCover}
            />
            <CreateProjectBasicFields
              name={name}
              author={author}
              tags={tags}
              labels={pd}
              onNameChange={setName}
              onAuthorChange={setAuthor}
              onTagsChange={setTags}
            />
          </div>

          <ProjectGenreSelector
            title={translations.projectType.selectType}
            projectGenres={projectGenres}
            genres={genres}
            onToggle={toggleGenre}
            onRemove={(genre) => setGenres((prev) => prev.filter((item) => item !== genre))}
          />

          <div className="dialog-actions">
            <ProjectDialogButton variant="outline" onClick={handleClose}>
              {translations.common.cancel}
            </ProjectDialogButton>
            <ProjectDialogButton
              onClick={handleConfirm}
              disabled={!name.trim() || !author.trim()}
            >
              {translations.common.confirm}
            </ProjectDialogButton>
          </div>
        </div>
      </ModalContent>
    </Modal>
  )
}

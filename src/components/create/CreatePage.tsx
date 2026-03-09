import { useEffect, useMemo, useState } from 'react'
import { Sparkles, BookOpen, AlignLeft, Wand2, ArrowRight } from 'lucide-react'
import { Swords, Rocket, Building2, SearchCode, Landmark } from 'lucide-react'

import { useTranslation } from '@/hooks/use-translation'
import { useToast } from '@/magic-ui/components'
import { useSettingsStore } from '@/stores/settings-store'
import { useProjectStore } from '@/stores/project-store'
import { useNavigationStore } from '@/stores/navigation-store'
import { createProjectFlow } from '@/components/home/page/home-page-project-actions-helpers'

import { GenreGrid } from './GenreGrid'
import { AiAssistBox } from './AiAssistBox'
import type { GenreOption, CreatePageProps } from './types'

const GENRE_ICONS = [Swords, Rocket, Building2, SearchCode, Landmark] as const

function buildGenreOptions(projectGenres: string[]): GenreOption[] {
  if (projectGenres.length === 0) return []
  return projectGenres.map((genre, index) => ({
    id: genre,
    name: genre,
    icon: GENRE_ICONS[index % GENRE_ICONS.length],
  }))
}

function resolveSelectedGenre(genres: GenreOption[], selectedGenre: string | null): string | null {
  if (genres.length === 0) return null
  if (!selectedGenre) return genres[0].id
  return genres.some((genre) => genre.id === selectedGenre) ? selectedGenre : genres[0].id
}

export function CreatePage({ onCreated }: CreatePageProps) {
  const { translations } = useTranslation()
  const { addToast } = useToast()
  const navigate = useNavigationStore((s) => s.navigate)
  const projectsRootDir = useSettingsStore((s) => s.projectsRootDir)
  const projectGenres = useSettingsStore((s) => s.projectGenres)
  const projectStore = useProjectStore()

  const cp = translations.createPage

  const genres = useMemo(() => buildGenreOptions(projectGenres), [projectGenres])

  const [title, setTitle] = useState('')
  const [selectedGenre, setSelectedGenre] = useState<string | null>(null)
  const [description, setDescription] = useState('')
  const [aiAssist, setAiAssist] = useState(true)
  const [titleError, setTitleError] = useState(false)
  const [submitting, setSubmitting] = useState(false)

  useEffect(() => {
    setSelectedGenre((prev) => resolveSelectedGenre(genres, prev))
  }, [genres])

  const handleCancel = () => {
    navigate('workspace')
  }

  const handleSubmit = async () => {
    const trimmedTitle = title.trim()
    if (!trimmedTitle) {
      setTitleError(true)
      return
    }

    setSubmitting(true)
    try {
      await createProjectFlow({
        onOpenSettings: () => navigate('settings'),
        projectsRootDir,
        projectStore,
        addToast,
        translations: {
          common: { error: translations.common.error },
          home: {
            configureRootDir: translations.home.configureRootDir,
            createSuccess: translations.home.createSuccess,
            projectCreatedMsg: translations.home.projectCreatedMsg,
          },
        },
        data: {
          name: trimmedTitle,
          author: '',
          tags: '',
          projectType: selectedGenre ? [selectedGenre] : [],
        },
      })

      const projectPath = `${projectsRootDir}/${trimmedTitle}`
      onCreated(projectPath)
    } catch (err) {
      addToast({
        title: translations.common.error,
        description: String(err),
        variant: 'destructive',
      })
    } finally {
      setSubmitting(false)
    }
  }

  return (
    <div className="content-scroll-create">
      <div className="creation-container">
        <div className="create-page-header">
          <h1 className="create-page-title">
            <Sparkles size={28} style={{ color: 'var(--color-primary-dark)' }} />
            {cp.pageTitle}
          </h1>
          <p className="create-page-subtitle">{cp.pageSubtitle}</p>
        </div>

        <div className="glass-panel-strong create-glass-card">
          <div className="create-form-group create-form-group-title">
            <input
              type="text"
              className={`create-title-input${titleError ? ' has-error' : ''}`}
              placeholder={cp.titlePlaceholder}
              value={title}
              onChange={(e) => {
                setTitle(e.target.value)
                if (titleError) setTitleError(false)
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') handleSubmit()
              }}
              autoFocus
              autoComplete="off"
            />
            {titleError && <p className="create-error-text">{cp.titleRequired}</p>}
          </div>

          <div className="create-form-group">
            <label className="create-form-label">
              <BookOpen size={18} style={{ color: 'var(--text-secondary)' }} />
              {cp.genreLabel}
            </label>
            {genres.length > 0 ? (
              <GenreGrid genres={genres} selected={selectedGenre} onSelect={setSelectedGenre} />
            ) : (
              <p className="create-error-text" style={{ textAlign: 'left', marginTop: 0 }}>
                {translations.settingsExtra.noGenres}
              </p>
            )}
          </div>

          <div className="create-form-group">
            <label className="create-form-label create-form-label-split">
              <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <AlignLeft size={18} style={{ color: 'var(--text-secondary)' }} />
                {cp.descLabel}
              </span>
              <button
                type="button"
                className="btn btn-link"
                style={{ fontSize: 13, fontWeight: 600, display: 'flex', alignItems: 'center', gap: 4 }}
              >
                <Wand2 size={14} /> {cp.aiExpand}
              </button>
            </label>
            <textarea
              className="create-desc-textarea"
              placeholder={cp.descPlaceholder}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>

          <div className="create-form-group">
            <AiAssistBox enabled={aiAssist} onToggle={() => setAiAssist((v) => !v)} />
          </div>

          <div className="create-action-bar">
            <button type="button" className="btn-large btn-outline-large" onClick={handleCancel}>
              {cp.cancel}
            </button>
            <button
              type="button"
              className="btn btn-large btn-primary-large"
              disabled={submitting}
              onClick={handleSubmit}
            >
              {submitting ? cp.submitting : translations.common.create}
              {!submitting && <ArrowRight size={20} />}
            </button>
          </div>
        </div>

        <div style={{ height: 40 }} />
      </div>
    </div>
  )
}
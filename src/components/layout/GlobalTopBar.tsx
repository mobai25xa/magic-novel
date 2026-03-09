import { useEffect, useState } from 'react'
import { Menu, Search, Sun, Moon, Bell, Plus, Trash2 } from 'lucide-react'

import { useTranslation, useT } from '@/hooks/use-translation'
import { useSettingsStore } from '@/state/settings'
import { GlobalSearchPanel } from '@/components/common/GlobalSearchPanel'
import { eventBus, EVENTS } from '@/lib/events'
import type { AppPage } from '@/types/navigation'

import { WindowControls } from './WindowControls'

interface GlobalTopBarProps {
  currentPage: AppPage
  onToggleSidebar: () => void
  onSearch: () => void
  onOpenCreate: () => void
  onOpenSkillCreate: () => void
}

export function GlobalTopBar({
  currentPage,
  onToggleSidebar,
  onSearch,
  onOpenCreate,
  onOpenSkillCreate,
}: GlobalTopBarProps) {
  const t = useT()
  const { translations } = useTranslation()
  const tw = translations.recyclePage
  const { theme, setTheme } = useSettingsStore()
  const [searchOpenPage, setSearchOpenPage] = useState<AppPage | null>(null)
  const [recycleSearch, setRecycleSearch] = useState('')

  const isRecyclePage = currentPage === 'recycle'

  const handleToggleTheme = () => {
    setTheme(theme === 'dark' ? 'light' : 'dark')
  }

  useEffect(() => {
    if (!isRecyclePage) return

    eventBus.emit(EVENTS.RECYCLE_SEARCH_CHANGED, recycleSearch)
  }, [isRecyclePage, recycleSearch])

  const handleSearchClick = () => {
    setSearchOpenPage(currentPage)
    onSearch()
  }

  const handleRecycleEmptyAll = () => {
    eventBus.emit(EVENTS.RECYCLE_EMPTY_ALL_REQUESTED)
  }

  const searchPlaceholder = isRecyclePage
    ? `${tw.searchPlaceholder || `${t('common.search')}${t('nav.recycle')}...`}`
    : t('nav.search')

  return (
    <header className="global-topbar">
      <div className="topbar-left" data-no-drag>
        <button className="icon-btn icon-btn-plain" onClick={onToggleSidebar}>
          <Menu size={24} />
        </button>

        <div
          className={`search-pill${isRecyclePage ? ' search-pill-recycle' : ''}`}
          role={isRecyclePage ? undefined : 'button'}
          tabIndex={isRecyclePage ? undefined : 0}
          onClick={isRecyclePage ? undefined : handleSearchClick}
          onKeyDown={isRecyclePage ? undefined : (event) => {
            if (event.key === 'Enter' || event.key === ' ') {
              event.preventDefault()
              handleSearchClick()
            }
          }}
        >
          <Search size={18} />
          <input
            type="text"
            placeholder={searchPlaceholder}
            value={isRecyclePage ? recycleSearch : ''}
            readOnly={!isRecyclePage}
            tabIndex={isRecyclePage ? undefined : -1}
            onChange={isRecyclePage ? (event) => setRecycleSearch(event.target.value) : undefined}
          />
          <kbd className="search-pill-kbd">⌘K</kbd>
        </div>
      </div>

      <div className="flex-1 h-full min-w-[48px]" data-tauri-drag-region />

      <div className="header-actions" data-no-drag>
        <button className="theme-toggle icon-btn" onClick={handleToggleTheme} title={t('settings.theme')}>
          {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
        </button>

        {!isRecyclePage && (
          <button className="icon-btn" title={t('common.info')}>
            <Bell size={18} />
          </button>
        )}

        {currentPage === 'workspace' ? (
          <>
            <div className="topbar-divider" />
            <button className="btn-create" onClick={onOpenCreate}>
              <Plus size={18} />
              {t('nav.newNovel')}
            </button>
          </>
        ) : null}

        {currentPage === 'skills' ? (
          <>
            <div className="topbar-divider" />
            <button className="topbar-skill-create" onClick={onOpenSkillCreate}>
              <Plus size={18} />
              {t('skillsWorkshop.createSkill')}
            </button>
          </>
        ) : null}

        {isRecyclePage ? (
          <>
            <div className="topbar-divider" />
            <button className="btn-danger-outline" onClick={handleRecycleEmptyAll}>
              <Trash2 size={18} />
              {tw.emptyAll}
            </button>
          </>
        ) : null}
      </div>

      <WindowControls />

      {searchOpenPage === currentPage && !isRecyclePage && (
        <div data-no-drag>
          <GlobalSearchPanel
            query=""
            isOpen={searchOpenPage === currentPage}
            onClose={() => setSearchOpenPage(null)}
            onResultClick={(_chapterPath) => setSearchOpenPage(null)}
          />
        </div>
      )}
    </header>
  )
}

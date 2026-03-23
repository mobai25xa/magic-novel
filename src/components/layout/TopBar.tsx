import { useState } from 'react'
import {
  Menu,
  Moon,
  Search,
  Settings,
  Sparkles,
  Sun,
} from 'lucide-react'

import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useSettingsStore } from '@/stores/settings-store'

import { openEditorTarget } from '@/features/editor-navigation/open-editor-target'
import { useTranslation } from '@/hooks/use-translation'
import { useLayoutStore } from '@/stores/layout-store'
import { GlobalSearchPanel } from '@/components/common/GlobalSearchPanel'
import { WindowControls } from './WindowControls'

interface TopBarProps {
  onOpenSettings: () => void
}

export function TopBar({ onOpenSettings }: TopBarProps) {
  const { projectPath } = useProjectStore()
  const { isSaving, isDirty } = useEditorStore()
  const { isLeftPanelVisible, isRightPanelVisible, toggleLeftPanel, toggleRightPanel } = useLayoutStore()
  const { theme, setTheme } = useSettingsStore()
  const { translations } = useTranslation()

  const [searchQuery, setSearchQuery] = useState('')
  const [showSearchResults, setShowSearchResults] = useState(false)

  const saveStatusText = isSaving
    ? translations.editor.saving
    : isDirty
      ? translations.editor.unsaved
      : translations.editor.saved

  const saveStatusClass = isSaving ? 'is-saving' : isDirty ? 'is-dirty' : 'is-saved'

  const handleToggleTheme = () => {
    setTheme(theme === 'dark' ? 'light' : 'dark')
  }

  return (
    <header className="editor-shell-topbar">
      <div className="editor-shell-topbar-left" data-no-drag>
        <button
          className="editor-shell-icon-btn"
          onClick={toggleLeftPanel}
          title={isLeftPanelVisible ? translations.layout.hideDirectory : translations.layout.showDirectory}
        >
          <Menu size={18} />
        </button>

        <div className={`editor-shell-status ${saveStatusClass}`}>
          <span className="editor-shell-status-dot" />
          {saveStatusText}
        </div>
      </div>

      <div className="editor-shell-search" role="search" data-no-drag>
        <Search size={14} className="editor-shell-search-icon" />
        <input
          type="text"
          placeholder={`${translations.common.search}...`}
          value={searchQuery}
          onChange={(event) => setSearchQuery(event.target.value)}
          onFocus={() => setShowSearchResults(true)}
          onBlur={() => setTimeout(() => setShowSearchResults(false), 200)}
        />
      </div>

      <div className="flex-1 h-full min-w-[48px]" data-tauri-drag-region />

      <div className="editor-shell-topbar-right" data-no-drag>
        <button
          className="editor-shell-icon-btn"
          onClick={handleToggleTheme}
          title={translations.settings.theme}
        >
          {theme === 'dark' ? <Sun size={18} /> : <Moon size={18} />}
        </button>

        <button
          className="editor-shell-icon-btn"
          onClick={onOpenSettings}
          title={translations.settings.title}
        >
          <Settings size={18} />
        </button>

        <div className="editor-shell-divider" />

        <button
          className={`editor-shell-ai-trigger${isRightPanelVisible ? ' active' : ''}`}
          onClick={toggleRightPanel}
          title={translations.layout.showAssistant}
        >
          <Sparkles size={16} />
          Magic
        </button>

        <WindowControls />
      </div>

      <div data-no-drag>
        <GlobalSearchPanel
          query={searchQuery}
          isOpen={showSearchResults && searchQuery.trim().length > 0}
          onClose={() => setShowSearchResults(false)}
          onResultClick={(path) => {
            if (!projectPath) return

            const open = async () => {
              const normalized = String(path || '').replace(/\\/g, '/')
              const opened = await openEditorTarget(normalized, {
                revealLeftTree: true,
                switchLeftTab: true,
              })
              if (!opened) {
                throw new Error(`Unsupported search target: ${normalized}`)
              }
              setShowSearchResults(false)
            }

            open().catch((error) => {
              console.error('Failed to open search result:', error)
            })
          }}
        />
      </div>
    </header>
  )
}

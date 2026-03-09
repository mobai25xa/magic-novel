import { useState } from 'react'
import {
  Menu,
  Moon,
  Search,
  Settings,
  Sparkles,
  Sun,
} from 'lucide-react'

import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useSettingsStore } from '@/stores/settings-store'

import { readMagicAsset } from '@/features/assets-management'
import {
  assetTreeToEditorDoc,
  type KnowledgeAssetTree,
} from '@/features/assets-management/asset-editor-document'
import { readChapter } from '@/features/editor-reading'
import { useTranslation } from '@/hooks/use-translation'
import { useLayoutStore } from '@/stores/layout-store'
import { GlobalSearchPanel } from '@/components/common/GlobalSearchPanel'
import { WindowControls } from './WindowControls'

interface TopBarProps {
  onOpenSettings: () => void
}

export function TopBar({ onOpenSettings }: TopBarProps) {
  const { projectPath, setSelectedPath } = useProjectStore()
  const { setCurrentChapter, setContent, setIsDirty, isSaving, isDirty } = useEditorStore()
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

            const normalized = String(path || '').replace('\\', '/')
            const isAsset = normalized.startsWith('.magic_novel/')

            const open = async () => {
              if (isAsset) {
                const relative = normalized.replace(/^\.magic_novel\//, '')
                const asset = (await readMagicAsset(projectPath, relative)) as KnowledgeAssetTree
                const title = asset && typeof asset === 'object' && 'title' in asset && typeof asset.title === 'string'
                  ? asset.title
                  : null
                const docContent = assetTreeToEditorDoc(asset)

                setSelectedPath(`magic_assets/${relative}`)
                const editorStore = useEditorStore.getState()
                editorStore.setCurrentAsset(relative, title)
                editorStore.setContent(docContent)
                editorStore.setIsDirty(false)
                setShowSearchResults(false)
                return
              }

              const chapter = await readChapter(projectPath, normalized)
              setSelectedPath(normalized)
              setCurrentChapter(chapter.id, normalized, chapter.title)
              setContent(chapter.content)
              setIsDirty(false)
              useEditorStore.getState().setLastOpened(projectPath, normalized, chapter.id, chapter.title)
              useAgentChatStore.getState().setActiveChapterPath(normalized)
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

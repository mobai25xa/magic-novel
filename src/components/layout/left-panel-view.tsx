import { useMemo, useState } from 'react'
import { ChevronLeft, ChevronRight, File, FileText, FolderPlus, Plus } from 'lucide-react'

import { InputDialog } from '@/components/common/InputDialog'
import { SelectDialog } from '@/components/common/SelectDialog'
import { ContentTree } from '@/components/tree/ContentTree'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useNavigationStore } from '@/stores/navigation-store'
import { TooltipProvider } from '@/magic-ui/components'

import {
  LeftPanelInputDialog,
  LeftPanelPinnedAssetsDialog,
  LeftPanelSelectVolumeDialog,
} from './left-panel-dialogs'

type LeftPanelTab = 'outline' | 'knowledge'

type LeftPanelTreeNode = {
  kind: 'dir' | 'chapter' | 'knowledge' | 'asset_dir' | 'asset_file'
  path: string
  name: string
  title?: string
  textLengthNoWhitespace?: number
  children?: LeftPanelTreeNode[]
  chapterId?: string
  assetRelativePath?: string
}

export type LeftPanelViewProps = {
  projectExists: boolean
  projectPath: string | null
  translations: {
    common: {
      back: string
    }
    home: {
      homePage: string
    }
    editor: {
      tableOfContents: string
      newVolume: string
      newChapter: string
      openOrCreateProject: string
      goalSetSuccess: string
      goalSetTo: string
      goalCleared: string
      goalSetFailed: string
      selectTargetVolume: string
      selectVolumeForChapter: string
      enterChapterName: string
      createVolumeFirst: string
    }
  }
  tree: LeftPanelTreeNode[]
  inputDialog: {
    open: boolean
    title: string
    placeholder: string
    onConfirm: (value: string) => void
  } | null
  selectVolumeDialog: {
    open: boolean
    chapterTitle: string
  } | null
  pinnedAssetsDialogOpen: boolean
  pinnedAssetsOptions: { value: string; label: string }[]
  pinnedAssetsDefault?: string
  onCreateVolume: () => void
  onCreateChapter: () => void
  onChapterSelect: (path: string, id: string, title?: string) => void
  onAssetSelect: (relativePath: string) => void
  onOpenPinnedAssetsDialog: () => void
  onCloseInputDialog: () => void
  onClosePinnedAssetsDialog: () => void
  onConfirmPinnedAssetsDialog: (value: string) => void
  onCloseSelectVolumeDialog: () => void
  onConfirmSelectVolumeDialog: (volumePath: string, chapterTitle: string) => void
  onCreateKnowledgeFolder: () => void
  onCreateKnowledgeFile: () => void
  knowledgeDialog:
    | {
        open: true
        kind: 'folder'
        title: string
        placeholder: string
        targetDir: string
        onConfirm: (name: string) => Promise<void>
      }
    | {
        open: true
        kind: 'file-type'
        title: string
        label: string
        targetDir: string
        options: { value: string; label: string }[]
        defaultValue: string
        onConfirm: (assetKind: 'worldview' | 'outline' | 'character' | 'lore' | 'prompt') => void
      }
    | {
        open: true
        kind: 'file-title'
        title: string
        placeholder: string
        targetDir: string
        assetKind: 'worldview' | 'outline' | 'character' | 'lore' | 'prompt'
        onConfirm: (name: string) => Promise<void>
      }
    | null
  onCloseKnowledgeDialog: () => void
}

type OutlineFileRowProps = {
  node: LeftPanelTreeNode
  currentChapterPath: string | null
  currentAssetPath: string | null
  onSelectChapter: (path: string, id: string, title?: string) => void
  onSelectAsset?: (relativePath: string) => void
}

function formatWordCount(value: number | undefined) {
  if (!value || value <= 0) return '0'
  if (value >= 1000) return `${(value / 1000).toFixed(1)}k`
  return String(value)
}

function OutlineFileRow({
  node,
  currentChapterPath,
  currentAssetPath,
  onSelectChapter,
  onSelectAsset,
}: OutlineFileRowProps) {
  const isChapterNode = node.kind === 'chapter'
  const isAssetNode = node.kind === 'asset_file'
  const activeAssetPath = node.assetRelativePath || null
  const isActive = isChapterNode
    ? currentChapterPath === node.path
    : isAssetNode
      ? activeAssetPath !== null && currentAssetPath === activeAssetPath
      : false

  const handleClick = () => {
    if (isChapterNode && node.chapterId) {
      onSelectChapter(node.path, node.chapterId, node.title)
      return
    }

    if (isAssetNode && node.assetRelativePath && onSelectAsset) {
      onSelectAsset(node.assetRelativePath)
    }
  }

  return (
    <button
      type="button"
      className={`editor-shell-outline-item${isActive ? ' is-active' : ''}`}
      onClick={handleClick}
      title={node.title || node.name}
    >
      <div className="editor-shell-outline-item-left">
        {isActive ? (
          <FileText size={14} className="editor-shell-outline-item-icon is-active" />
        ) : (
          <File size={14} className="editor-shell-outline-item-icon" />
        )}
        <span className="editor-shell-outline-item-title">{node.title || node.name}</span>
      </div>
      {isChapterNode ? (
        <span className={`editor-shell-outline-item-count${isActive ? ' is-active' : ''}`}>
          {formatWordCount(node.textLengthNoWhitespace)}
        </span>
      ) : null}
    </button>
  )
}

type OutlineVolumeBlockProps = {
  node: LeftPanelTreeNode
  currentChapterPath: string | null
  currentAssetPath: string | null
  onSelectChapter: (path: string, id: string, title?: string) => void
  onSelectAsset?: (relativePath: string) => void
}

function OutlineVolumeBlock({
  node,
  currentChapterPath,
  currentAssetPath,
  onSelectChapter,
  onSelectAsset,
}: OutlineVolumeBlockProps) {
  const [open, setOpen] = useState(true)
  const chapterChildren = (node.children || []).filter((child) => child.kind === 'chapter' || child.kind === 'asset_file')

  return (
    <div className="editor-shell-volume-group">
      <button
        type="button"
        className="editor-shell-volume-header"
        onClick={() => setOpen((prev) => !prev)}
      >
        {open ? <ChevronRight size={12} className="editor-shell-volume-arrow is-open" /> : <ChevronRight size={12} className="editor-shell-volume-arrow" />}
        <span className="editor-shell-volume-title">{node.title || node.name}</span>
        <span className="editor-shell-volume-count">{chapterChildren.length}</span>
      </button>

      {open ? (
        <div className="editor-shell-volume-children">
          {chapterChildren.map((chapter) => (
            <OutlineFileRow
              key={chapter.path}
              node={chapter}
              currentChapterPath={currentChapterPath}
              currentAssetPath={currentAssetPath}
              onSelectChapter={onSelectChapter}
              onSelectAsset={onSelectAsset}
            />
          ))}
        </div>
      ) : null}
    </div>
  )
}

function OutlineContent(input: LeftPanelViewProps) {
  const { currentChapterPath, currentAssetPath } = useEditorStore()
  const volumes = useMemo(
    () => input.tree.filter((node) => node.kind === 'dir'),
    [input.tree],
  )

  if (!input.projectExists) {
    return (
      <div className="editor-shell-outline-empty">{input.translations.editor.openOrCreateProject}</div>
    )
  }

  if (volumes.length === 0) {
    return (
      <div className="editor-shell-outline-scroll">
        <div className="editor-shell-outline-empty">
          <div>{input.translations.editor.createVolumeFirst}</div>
          <button
            type="button"
            className="editor-shell-left-create"
            onClick={input.onCreateVolume}
            style={{ marginTop: 10 }}
          >
            <FolderPlus size={14} />
            {input.translations.editor.newVolume}
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="editor-shell-outline-scroll">
      {volumes.map((volume) => (
        <OutlineVolumeBlock
          key={volume.path}
          node={volume}
          currentChapterPath={currentChapterPath}
          currentAssetPath={currentAssetPath}
          onSelectChapter={input.onChapterSelect}
          onSelectAsset={input.onAssetSelect}
        />
      ))}
    </div>
  )
}

function resolveProjectDisplayName(projectName: string | undefined, projectPath: string | null) {
  if (projectName && projectName.trim().length > 0) {
    return projectName
  }

  if (!projectPath) {
    return 'Magic Novel'
  }

  const normalizedPath = projectPath.replace(/\\/g, '/').replace(/\/+$/, '')
  const fallbackName = normalizedPath.split('/').pop() || 'Magic Novel'

  try {
    return decodeURIComponent(fallbackName)
  } catch {
    return fallbackName
  }
}

function SidebarHeader(input: Pick<LeftPanelViewProps, 'translations'>) {
  const { project, projectPath } = useProjectStore()
  const navigate = useNavigationStore((state) => state.navigate)
  const projectDisplayName = resolveProjectDisplayName(project?.name, projectPath)

  return (
    <div className="editor-shell-left-header">
      <button
        type="button"
        className="editor-shell-left-back"
        onClick={() => navigate('workspace')}
      >
        <ChevronLeft size={14} className="editor-shell-left-back-arrow" />
        {input.translations.common.back}{input.translations.home.homePage}
      </button>

      <div className="editor-shell-book-info">
        <img src="/icon.png" alt="Magic Novel" className="editor-shell-book-icon" />
        <h2 className="editor-shell-book-title">{projectDisplayName}</h2>
      </div>
    </div>
  )
}

function SidebarTabs(input: {
  activeTab: LeftPanelTab
  setActiveTab: (tab: LeftPanelTab) => void
}) {
  return (
    <div className="editor-shell-left-tabs">
      <button
        type="button"
        className={`editor-shell-left-tab${input.activeTab === 'outline' ? ' is-active' : ''}`}
        onClick={() => input.setActiveTab('outline')}
      >
        大纲
      </button>
      <button
        type="button"
        className={`editor-shell-left-tab${input.activeTab === 'knowledge' ? ' is-active' : ''}`}
        onClick={() => input.setActiveTab('knowledge')}
      >
        知识库
      </button>
    </div>
  )
}

function SidebarFooter(input: LeftPanelViewProps & { activeTab: LeftPanelTab }) {
  if (input.activeTab === 'knowledge') {
    return (
      <div className="editor-shell-left-footer">
        <div className="editor-shell-left-footer-actions">
          <button
            type="button"
            className="editor-shell-left-create"
            onClick={input.onCreateKnowledgeFolder}
          >
            <FolderPlus size={14} />
            新建文件夹
          </button>

          <button
            type="button"
            className="editor-shell-left-create"
            onClick={input.onCreateKnowledgeFile}
          >
            <Plus size={14} />
            新建文件
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="editor-shell-left-footer">
      <div className="editor-shell-left-footer-actions">
        <button
          type="button"
          className="editor-shell-left-create"
          onClick={input.onCreateVolume}
        >
          <FolderPlus size={14} />
          {input.translations.editor.newVolume}
        </button>

        <button
          type="button"
          className="editor-shell-left-create"
          onClick={input.onCreateChapter}
        >
          <Plus size={14} />
          {input.translations.editor.newChapter}
        </button>
      </div>
    </div>
  )
}

function KnowledgeContent(input: LeftPanelViewProps) {
  const { currentChapterPath } = useEditorStore()

  if (!input.projectPath) {
    return <div className="editor-shell-outline-empty">{input.translations.editor.openOrCreateProject}</div>
  }

  return (
    <ContentTree
      mode="knowledge"
      hideKnowledgeRoot
      variant="outline"
      onChapterSelect={input.onChapterSelect}
      onCreateChapterInVolume={undefined}
      onAssetSelect={input.onAssetSelect}
      key={`knowledge-${input.projectPath}-${currentChapterPath || 'none'}`}
    />
  )
}

function LeftPanelKnowledgeDialogs(input: LeftPanelViewProps) {
  if (!input.knowledgeDialog) return null

  if (input.knowledgeDialog.kind === 'file-type') {
    return (
      <SelectDialog
        open={input.knowledgeDialog.open}
        title={input.knowledgeDialog.title}
        label={input.knowledgeDialog.label}
        options={input.knowledgeDialog.options}
        defaultValue={input.knowledgeDialog.defaultValue}
        closeOnConfirm={false}
        onClose={input.onCloseKnowledgeDialog}
        onConfirm={(value) => {
          if (input.knowledgeDialog?.kind !== 'file-type') return
          input.knowledgeDialog.onConfirm(value as 'worldview' | 'outline' | 'character' | 'lore' | 'prompt')
        }}
      />
    )
  }

  if (input.knowledgeDialog.kind === 'folder') {
    return (
      <InputDialog
        open={input.knowledgeDialog.open}
        title={input.knowledgeDialog.title}
        placeholder={input.knowledgeDialog.placeholder}
        onClose={input.onCloseKnowledgeDialog}
        onConfirm={(value) => {
          if (input.knowledgeDialog?.kind !== 'folder') return
          void input.knowledgeDialog.onConfirm(value)
        }}
      />
    )
  }

  return (
    <InputDialog
      open={input.knowledgeDialog.open}
      title={input.knowledgeDialog.title}
      placeholder={input.knowledgeDialog.placeholder}
      onClose={input.onCloseKnowledgeDialog}
      onConfirm={(value) => {
        if (input.knowledgeDialog?.kind !== 'file-title') return
        void input.knowledgeDialog.onConfirm(value)
      }}
    />
  )
}

function LeftPanelDialogs(input: LeftPanelViewProps) {
  return (
    <>
      <LeftPanelInputDialog state={input.inputDialog} onClose={input.onCloseInputDialog} />

      <LeftPanelPinnedAssetsDialog
        open={input.pinnedAssetsDialogOpen}
        options={input.pinnedAssetsOptions}
        defaultValue={input.pinnedAssetsDefault}
        onClose={input.onClosePinnedAssetsDialog}
        onConfirm={input.onConfirmPinnedAssetsDialog}
      />

      <LeftPanelSelectVolumeDialog
        state={input.selectVolumeDialog}
        title={input.translations.editor.selectTargetVolume}
        label={input.translations.editor.selectVolumeForChapter}
        options={input.tree
          .filter((node) => node.kind === 'dir')
          .map((node) => ({ value: node.path, label: node.title || node.name }))}
        onClose={input.onCloseSelectVolumeDialog}
        onConfirm={input.onConfirmSelectVolumeDialog}
      />

      <LeftPanelKnowledgeDialogs {...input} />
    </>
  )
}

export function LeftPanelView(input: LeftPanelViewProps) {
  const [activeTab, setActiveTab] = useState<LeftPanelTab>('outline')

  return (
    <TooltipProvider>
      <div className="panel-sidebar editor-shell-left-panel" data-no-drag>
        <SidebarHeader translations={input.translations} />
        <SidebarTabs activeTab={activeTab} setActiveTab={setActiveTab} />

        {activeTab === 'outline' ? (
          <OutlineContent {...input} />
        ) : (
          <div className="editor-shell-outline-scroll editor-shell-kb-wrap">
            <KnowledgeContent {...input} />
          </div>
        )}

        <SidebarFooter {...input} activeTab={activeTab} />
        <LeftPanelDialogs {...input} />
      </div>
    </TooltipProvider>
  )
}

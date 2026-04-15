import { useMemo } from 'react'
import { ChevronLeft, FolderPlus, Plus } from 'lucide-react'

import { InputDialog } from '@/components/common/InputDialog'
import { ContentTree } from '@/components/tree/ContentTree'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useNavigationStore } from '@/stores/navigation-store'
import { useEditorUiStore, type LeftPanelTab } from '@/stores/editor-ui-store'
import { TooltipProvider } from '@/magic-ui/components'

import {
  LeftPanelInputDialog,
  LeftPanelPinnedAssetsDialog,
  LeftPanelSelectVolumeDialog,
} from './left-panel-dialogs'

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
      projectHome: string
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
  onCreateChapterInVolumeDirect: (volumePath: string) => void
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
        kind: 'file'
        title: string
        placeholder: string
        targetDir: string
        onConfirm: (name: string) => Promise<void>
      }
    | null
  onCloseKnowledgeDialog: () => void
}

function OutlineContent(input: LeftPanelViewProps) {
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
      <ContentTree
        mode="manuscript"
        variant="outline"
        onChapterSelect={input.onChapterSelect}
        onCreateChapterInVolume={input.onCreateChapterInVolumeDirect}
        onAssetSelect={input.onAssetSelect}
      />
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
        onClick={() => navigate(projectPath ? 'project_home' : 'workspace')}
      >
        <ChevronLeft size={14} className="editor-shell-left-back-arrow" />
        {input.translations.common.back}{projectPath ? input.translations.home.projectHome : input.translations.home.homePage}
      </button>

      <div className="editor-shell-book-info">
        <img src="/icon.png" alt="Magic Novel" className="editor-shell-book-icon" />
        <h2 className="editor-shell-book-title">{projectDisplayName}</h2>
      </div>
    </div>
  )
}

function SidebarTabs(input: {
  translations: Pick<LeftPanelViewProps['translations'], 'editor'>
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
        {input.translations.editor.tableOfContents}
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
        if (input.knowledgeDialog?.kind !== 'file') return
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
  const activeTab = useEditorUiStore((state) => state.leftPanelTab)
  const setActiveTab = useEditorUiStore((state) => state.setLeftPanelTab)

  return (
    <TooltipProvider>
      <div className="panel-sidebar editor-shell-left-panel" data-no-drag>
        <SidebarHeader translations={input.translations} />
        <SidebarTabs translations={input.translations} activeTab={activeTab} setActiveTab={setActiveTab} />

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

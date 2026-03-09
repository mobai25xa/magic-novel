import { useEffect, useRef } from 'react'
import { ChevronLeft, FolderPlus, Plus } from 'lucide-react'

import { Button, Textarea } from '@/magic-ui/components'

import type { AssetKind } from '@/features/assets-management'

import type { AssetNode, AssetTree } from './asset-tree-utils'

export type AssetExplorerEntry = {
  kind: 'dir' | 'file'
  path: string
  title: string
  subtitle?: string
}

const ASSET_CATEGORIES: { kind: AssetKind; label: string }[] = [
  { kind: 'worldview', label: '世界观' },
  { kind: 'outline', label: '大纲' },
  { kind: 'character', label: '人物' },
  { kind: 'lore', label: '资料' },
  { kind: 'prompt', label: '提示词' },
]

type AssetManagerViewProps = {
  projectPath: string | null
  kind: AssetKind
  currentFolderLabel: string
  canGoParent: boolean
  isLoading: boolean
  explorerEntries: AssetExplorerEntry[]
  selectedAssetPath: string | null
  asset: AssetTree | null
  nodes: AssetNode[]
  selectedNode: AssetNode | null
  selectedNodeId: string | null
  nodeContentDraft: string
  createDraftKind: 'folder' | 'file' | null
  createDraftValue: string
  onKindChange: (kind: AssetKind) => void
  onOpenEntry: (entry: AssetExplorerEntry) => void
  onGoParent: () => void
  onNodeSelect: (id: string) => void
  onNodeDraftChange: (value: string) => void
  onCreateDraftValueChange: (value: string) => void
  onConfirmCreateDraft: () => Promise<void>
  onCancelCreateDraft: () => void
  onStartCreateFile: () => void
  onStartCreateFolder: () => void
  onSaveSelectedNode: () => Promise<void>
}

function AssetCategorySection(input: {
  kind: AssetKind
  activeKind: AssetKind
  onKindChange: (kind: AssetKind) => void
}) {
  const active = input.activeKind === input.kind
  const label = ASSET_CATEGORIES.find((category) => category.kind === input.kind)?.label || input.kind

  return (
    <button
      className={`sidebar-tab text-sm ${active ? 'sidebar-tab-active' : ''}`}
      onClick={() => input.onKindChange(input.kind)}
    >
      {label}
    </button>
  )
}

function ExplorerPane(input: {
  isLoading: boolean
  currentFolderLabel: string
  canGoParent: boolean
  explorerEntries: AssetExplorerEntry[]
  selectedAssetPath: string | null
  createDraftKind: 'folder' | 'file' | null
  createDraftValue: string
  onOpenEntry: (entry: AssetExplorerEntry) => void
  onGoParent: () => void
  onCreateDraftValueChange: (value: string) => void
  onConfirmCreateDraft: () => Promise<void>
  onCancelCreateDraft: () => void
}) {
  const createPlaceholder = input.createDraftKind === 'folder' ? '新文件夹名称' : '新文件名称'
  const createInputRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => {
    if (!input.createDraftKind) return
    const node = createInputRef.current
    if (!node) return
    node.focus()
    node.select()
  }, [input.createDraftKind])

  return (
    <div className="panel-sidebar overflow-auto" style={{ width: '320px' }}>
      <div className="p-3 border-b flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="text-sm font-medium truncate">{input.currentFolderLabel}</div>
          <div className="text-xs text-muted-foreground">
            {input.isLoading ? '加载中...' : `${input.explorerEntries.length} 项`}
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          disabled={!input.canGoParent}
          onClick={input.onGoParent}
          title="返回上一级"
        >
          <ChevronLeft size={14} />
        </Button>
      </div>

      {input.createDraftKind ? (
        <div className="sidebar-tab">
          <input
            ref={createInputRef}
            autoFocus
            value={input.createDraftValue}
            onChange={(event) => input.onCreateDraftValueChange(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault()
                event.currentTarget.blur()
              }
              if (event.key === 'Escape') {
                event.preventDefault()
                input.onCancelCreateDraft()
              }
            }}
            onBlur={() => {
              if (!input.createDraftValue.trim()) {
                input.onCancelCreateDraft()
                return
              }
              void input.onConfirmCreateDraft().catch(() => undefined)
            }}
            className="w-full h-8 rounded-md border border-[var(--border-focus)] bg-[var(--bg-white)] px-2 text-sm outline-none"
            placeholder={createPlaceholder}
          />
        </div>
      ) : null}

      {input.explorerEntries.map((entry) => {
        const selected = entry.kind === 'file' && input.selectedAssetPath === entry.path

        return (
          <button
            key={entry.path}
            className={`sidebar-tab text-sm ${selected ? 'sidebar-tab-active' : ''}`}
            onClick={() => input.onOpenEntry(entry)}
          >
            <div className="font-medium truncate">{entry.title}</div>
            {entry.subtitle ? <div className="text-xs text-muted-foreground truncate">{entry.subtitle}</div> : null}
          </button>
        )
      })}

      {!input.isLoading && input.explorerEntries.length === 0 ? (
        <div className="px-4 py-3 text-sm text-muted-foreground">当前目录暂无内容。</div>
      ) : null}
    </div>
  )
}

function AssetNodeList(input: {
  nodes: AssetNode[]
  selectedNodeId: string | null
  asset: AssetTree | null
  onNodeSelect: (id: string) => void
}) {
  return (
    <div className="panel-sidebar overflow-auto" style={{ width: '288px' }}>
      <div className="p-2 text-xs text-muted-foreground">结构</div>
      {input.nodes.map((node) => (
        <button
          key={node.node_id}
          className={`sidebar-tab text-sm ${input.selectedNodeId === node.node_id ? 'sidebar-tab-active' : ''}`}
          style={{ paddingLeft: `${12 + Math.max(0, node.level - 1) * 12}px` }}
          onClick={() => input.onNodeSelect(node.node_id)}
        >
          {node.title || '(无标题)'}
        </button>
      ))}
      {input.nodes.length === 0 && input.asset ? (
        <div className="p-3 text-sm text-muted-foreground">该资产没有标题节点（可用 Markdown 标题重新导入）。</div>
      ) : null}
    </div>
  )
}

function KnowledgeBottomActions(input: {
  onStartCreateFolder: () => void
  onStartCreateFile: () => void
}) {
  return (
    <div className="editor-shell-left-footer">
      <div className="editor-shell-left-footer-actions">
        <button type="button" className="editor-shell-left-create" onClick={input.onStartCreateFolder}>
          <FolderPlus size={14} />
          新建文件夹
        </button>
        <button type="button" className="editor-shell-left-create" onClick={input.onStartCreateFile}>
          <Plus size={14} />
          新建文件
        </button>
      </div>
    </div>
  )
}

export function AssetManagerView(input: AssetManagerViewProps) {
  if (!input.projectPath) {
    return <div className="p-4 text-sm text-muted-foreground">请先打开作品</div>
  }

  const currentKindLabel = ASSET_CATEGORIES.find((category) => category.kind === input.kind)?.label || input.kind

  return (
    <div className="h-full flex flex-col">
      <div className="flex-1 flex min-h-0">
        <div className="panel-sidebar overflow-auto" style={{ width: '220px' }}>
          <div className="p-3 border-b">
            <div className="text-sm font-medium">知识库</div>
            <div className="text-xs text-muted-foreground">当前分类：{currentKindLabel}</div>
          </div>

          {ASSET_CATEGORIES.map((category) => (
            <AssetCategorySection
              key={category.kind}
              kind={category.kind}
              activeKind={input.kind}
              onKindChange={input.onKindChange}
            />
          ))}
        </div>

        <ExplorerPane
          isLoading={input.isLoading}
          currentFolderLabel={input.currentFolderLabel}
          canGoParent={input.canGoParent}
          explorerEntries={input.explorerEntries}
          selectedAssetPath={input.selectedAssetPath}
          createDraftKind={input.createDraftKind}
          createDraftValue={input.createDraftValue}
          onOpenEntry={input.onOpenEntry}
          onGoParent={input.onGoParent}
          onCreateDraftValueChange={input.onCreateDraftValueChange}
          onConfirmCreateDraft={input.onConfirmCreateDraft}
          onCancelCreateDraft={input.onCancelCreateDraft}
        />

        <div className="flex-1 flex min-h-0">
          <AssetNodeList
            nodes={input.nodes}
            selectedNodeId={input.selectedNodeId}
            asset={input.asset}
            onNodeSelect={input.onNodeSelect}
          />

          <div className="flex-1 flex flex-col min-h-0">
            <div className="p-3 border-b flex items-center justify-between">
              <div className="min-w-0">
                <div className="text-sm font-medium truncate">{input.asset?.title || '资产'}</div>
                {input.selectedNode ? (
                  <div className="text-xs text-muted-foreground truncate">node_id: {input.selectedNode.node_id}</div>
                ) : null}
              </div>
              <Button
                onClick={input.onSaveSelectedNode}
                size="sm"
                disabled={!input.asset || !input.selectedNode}
              >
                保存
              </Button>
            </div>
            <div className="flex-1 overflow-auto p-3">
              <Textarea
                value={input.nodeContentDraft}
                onChange={(event) => input.onNodeDraftChange(event.target.value)}
                placeholder="编辑该节点内容..."
                className="min-h-[240px]"
              />
            </div>
          </div>
        </div>
      </div>

      <KnowledgeBottomActions
        onStartCreateFolder={input.onStartCreateFolder}
        onStartCreateFile={input.onStartCreateFile}
      />
    </div>
  )
}

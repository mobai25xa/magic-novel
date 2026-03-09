import { Textarea } from '@/magic-ui/components'
import { Button } from '@/magic-ui/components'
import type { Translations } from '@/i18n/locales/zh'

import { flattenAssetNodes, type AssetTree } from './asset-tree-utils'

type AssetEditorViewProps = {
  asset: AssetTree | null
  selectedNodeId: string | null
  nodeContentDraft: string
  isLoading: boolean
  projectPath: string | null
  currentAssetPath: string | null
  onSelectNode: (nodeId: string) => void
  onSaveSelectedNode: () => void
  onNodeContentChange: (value: string) => void
  labels: Translations['assets']
}

export function AssetEditorView(input: AssetEditorViewProps) {
  if (!input.projectPath || !input.currentAssetPath) {
    return (
      <div className="flex-1 flex items-center justify-center bg-background">
        <div className="text-muted-foreground text-center">
          <p className="text-lg mb-2">{input.labels.selectFileToEdit}</p>
        </div>
      </div>
    )
  }

  if (input.isLoading) {
    return (
      <div className="flex-1 flex items-center justify-center bg-background">
        <div className="text-muted-foreground">加载中...</div>
      </div>
    )
  }

  const nodes = input.asset ? flattenAssetNodes(input.asset.root).filter((node) => node.level > 0) : []
  const selectedNode = input.asset && input.selectedNodeId
    ? flattenAssetNodes(input.asset.root).find((node) => node.node_id === input.selectedNodeId) || null
    : null

  return (
    <div className="flex-1 overflow-hidden flex bg-background">
      <div className="panel-sidebar overflow-auto" style={{ width: '288px' }}>
        <div className="p-3 border-b">
          <div className="text-sm font-medium truncate">{input.asset?.title || input.labels.knowledgeBase}</div>
          <div className="text-xs text-muted-foreground truncate">{input.currentAssetPath}</div>
        </div>

        {nodes.map((node) => (
          <button
            key={node.node_id}
            className={`sidebar-tab text-sm ${input.selectedNodeId === node.node_id ? 'sidebar-tab-active' : ''}`}
            style={{ paddingLeft: `${12 + Math.max(0, node.level - 1) * 12}px` }}
            onClick={() => input.onSelectNode(node.node_id)}
          >
            {node.title || input.labels.nodes}
          </button>
        ))}

        {nodes.length === 0 ? (
          <div className="p-3 text-sm text-muted-foreground">{input.labels.noTitleHint}</div>
        ) : null}
      </div>

      <div className="flex-1 flex flex-col min-h-0">
        <div className="p-3 border-b flex items-center justify-between">
          <div className="min-w-0">
            <div className="text-sm font-medium truncate">{selectedNode?.title || input.labels.nodes}</div>
            {selectedNode ? <div className="text-xs text-muted-foreground truncate">node_id: {selectedNode.node_id}</div> : null}
          </div>
          <Button
            onClick={input.onSaveSelectedNode}
            size="sm"
            disabled={!selectedNode}
          >
            保存
          </Button>
        </div>

        <div className="flex-1 overflow-auto p-3">
          <Textarea
            value={input.nodeContentDraft}
            onChange={(event) => input.onNodeContentChange(event.target.value)}
            placeholder={input.labels.editPlaceholder}
            className="min-h-[240px]"
          />
        </div>
      </div>
    </div>
  )
}

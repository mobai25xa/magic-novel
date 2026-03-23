import { useCallback, useState } from 'react'
import { saveAssetFile } from '@/features/assets-management'
import { useProjectStore } from '@/stores/project-store'
import { useEditorStore } from '@/stores/editor-store'
import { useToast } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'
import { flattenAssetNodes, type AssetTree, updateAssetNodeContent } from './asset-tree-utils'
import { useLoadAssetEditor } from './asset-editor-hooks'
import { AssetEditorView } from './asset-editor-view'

export function AssetEditor() {
  const { projectPath } = useProjectStore()
  const { currentAssetPath } = useEditorStore()
  const { addToast } = useToast()
  const { translations } = useTranslation()

  const [asset, setAsset] = useState<AssetTree | null>(null)
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)
  const [nodeContentDraft, setNodeContentDraft] = useState('')
  const [isLoading, setIsLoading] = useState(false)

  useLoadAssetEditor({
    projectPath,
    currentAssetPath,
    addToast,
    setAsset,
    setSelectedNodeId,
    setNodeContentDraft,
    setIsLoading,
  })

  const selectNode = useCallback(
    (nodeId: string) => {
      setSelectedNodeId(nodeId)
      if (!asset) {
        setNodeContentDraft('')
        return
      }

      const node = flattenAssetNodes(asset.root).find((item) => item.node_id === nodeId) || null
      setNodeContentDraft(node?.content || '')
    },
    [asset],
  )

  const saveSelectedNode = useCallback(async () => {
    if (!projectPath || !currentAssetPath || !asset || !selectedNodeId) return

    try {
      const nextAsset = updateAssetNodeContent(asset, selectedNodeId, nodeContentDraft)
      await saveAssetFile(projectPath, currentAssetPath, nextAsset)
      setAsset(nextAsset)
      addToast({ title: '保存成功', description: '知识库已更新', variant: 'success' })
    } catch (e) {
      console.error(e)
      addToast({ title: '保存失败', description: String(e), variant: 'destructive' })
    }
  }, [addToast, asset, currentAssetPath, nodeContentDraft, projectPath, selectedNodeId])

  return (
    <AssetEditorView
      asset={asset}
      selectedNodeId={selectedNodeId}
      nodeContentDraft={nodeContentDraft}
      isLoading={isLoading}
      projectPath={projectPath}
      currentAssetPath={currentAssetPath}
      onSelectNode={selectNode}
      onSaveSelectedNode={saveSelectedNode}
      onNodeContentChange={setNodeContentDraft}
      labels={translations.assets}
    />
  )
}

import { useEffect } from 'react'

import { readAssetFile } from '@/features/assets-management'

import { flattenAssetNodes, type AssetTree } from './asset-tree-utils'

type LoadAssetEditorInput = {
  projectPath: string | null
  currentAssetPath: string | null
  addToast: (toast: { title: string; description?: string; variant?: 'default' | 'success' | 'warning' | 'destructive' | 'info' }) => void
  setAsset: (asset: AssetTree | null) => void
  setSelectedNodeId: (id: string | null) => void
  setNodeContentDraft: (text: string) => void
  setIsLoading: (value: boolean) => void
}

export function useLoadAssetEditor({
  projectPath,
  currentAssetPath,
  addToast,
  setAsset,
  setSelectedNodeId,
  setNodeContentDraft,
  setIsLoading,
}: LoadAssetEditorInput) {
  useEffect(() => {
    const load = async () => {
      if (!projectPath || !currentAssetPath) return

      setIsLoading(true)
      try {
        const asset = (await readAssetFile(projectPath, currentAssetPath)) as AssetTree
        setAsset(asset)

        const firstNode = flattenAssetNodes(asset.root).find((node) => node.level > 0) || null
        setSelectedNodeId(firstNode?.node_id || null)
        setNodeContentDraft(firstNode?.content || '')
      } catch (error) {
        console.error(error)
        addToast({ title: '打开失败', description: String(error), variant: 'destructive' })
      } finally {
        setIsLoading(false)
      }
    }

    load()
  }, [
    addToast,
    currentAssetPath,
    projectPath,
    setAsset,
    setIsLoading,
    setNodeContentDraft,
    setSelectedNodeId,
  ])
}

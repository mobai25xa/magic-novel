import { useCallback, useEffect, useMemo, useState } from 'react'

import { useProjectStore } from '@/stores/project-store'
import {
  createAssetFile,
  createAssetFolder,
  getAssetsTree,
  readAssetFile,
  saveAssetFile,
  type AssetKind,
  type AssetLibraryNode,
} from '@/features/assets-management'
import { useToast } from '@/magic-ui/components'

import {
  findAssetNodeById,
  getAssetDisplayNodes,
  type AssetTree,
  updateAssetNodeContent,
} from './asset-tree-utils'
import { AssetManagerView, type AssetExplorerEntry } from './asset-manager-view'

type DirNode = Extract<AssetLibraryNode, { kind: 'dir' }>

const DEFAULT_KIND: AssetKind = 'worldview'

const ASSET_KIND_LABELS: Record<AssetKind, string> = {
  worldview: '世界观',
  outline: '大纲',
  character: '人物',
  lore: '资料',
  prompt: '提示词',
}

function normalizePath(value: string) {
  return String(value || '').replace(/\\/g, '/').replace(/\/+/g, '/')
}

function isDirNode(node: AssetLibraryNode): node is DirNode {
  return node.kind === 'dir'
}

function getNodeTitle(node: AssetLibraryNode) {
  if (node.kind === 'dir') return node.title || node.name
  return node.title || node.name
}

function findKindRoot(nodes: AssetLibraryNode[], kind: AssetKind): DirNode | null {
  const hit = nodes.find((node) => isDirNode(node) && normalizePath(node.path) === kind)
  return hit && isDirNode(hit) ? hit : null
}

function findDirByPath(root: DirNode, path: string): DirNode | null {
  const target = normalizePath(path)
  if (normalizePath(root.path) === target) return root

  for (const child of root.children) {
    if (!isDirNode(child)) continue
    const found = findDirByPath(child, target)
    if (found) return found
  }

  return null
}

function collectFilePaths(root: DirNode): Set<string> {
  const files = new Set<string>()

  const walk = (dir: DirNode) => {
    for (const child of dir.children) {
      if (isDirNode(child)) {
        walk(child)
      } else {
        files.add(normalizePath(child.path))
      }
    }
  }

  walk(root)
  return files
}

function parentPath(path: string) {
  const parts = normalizePath(path).split('/')
  parts.pop()
  return parts.join('/')
}

function mapChildrenToExplorerEntries(children: AssetLibraryNode[]): AssetExplorerEntry[] {
  const entries = children.map((node) => ({
    kind: node.kind,
    path: normalizePath(node.path),
    title: getNodeTitle(node),
    subtitle: node.kind === 'file' ? normalizePath(node.path) : undefined,
  }))

  entries.sort((a, b) => {
    if (a.kind !== b.kind) {
      return a.kind === 'dir' ? -1 : 1
    }
    return a.title.localeCompare(b.title)
  })

  return entries
}

export function AssetManager() {
  const { projectPath } = useProjectStore()
  const { addToast } = useToast()

  const [kind, setKind] = useState<AssetKind>(DEFAULT_KIND)
  const [kindRoot, setKindRoot] = useState<DirNode | null>(null)
  const [isLoadingTree, setIsLoadingTree] = useState(false)
  const [reloadSignal, setReloadSignal] = useState(0)

  const [explorerPath, setExplorerPath] = useState<string>(DEFAULT_KIND)
  const [selectedAssetPath, setSelectedAssetPath] = useState<string | null>(null)

  const [asset, setAsset] = useState<AssetTree | null>(null)
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)
  const [nodeContentDraft, setNodeContentDraft] = useState('')
  const [isLoadingAsset, setIsLoadingAsset] = useState(false)

  const [createDraftKind, setCreateDraftKind] = useState<'folder' | 'file' | null>(null)
  const [createDraftValue, setCreateDraftValue] = useState('')

  useEffect(() => {
    const loadTree = async () => {
      if (!projectPath) {
        setKindRoot(null)
        setExplorerPath(kind)
        setSelectedAssetPath(null)
        return
      }

      setIsLoadingTree(true)
      try {
        const nodes = await getAssetsTree(projectPath)
        const root = findKindRoot(nodes, kind)
        setKindRoot(root)

        if (!root) {
          setExplorerPath(kind)
          setSelectedAssetPath(null)
          return
        }

        setExplorerPath((prev) => {
          const normalizedPrev = normalizePath(prev)
          if (!normalizedPrev.startsWith(`${kind}`)) return kind
          return findDirByPath(root, normalizedPrev) ? normalizedPrev : kind
        })

        const fileSet = collectFilePaths(root)
        setSelectedAssetPath((prev) => {
          if (!prev) return null
          const normalizedPrev = normalizePath(prev)
          return fileSet.has(normalizedPrev) ? normalizedPrev : null
        })
      } catch (error) {
        console.error('Failed to load knowledge tree:', error)
        setKindRoot(null)
      } finally {
        setIsLoadingTree(false)
      }
    }

    void loadTree()
  }, [kind, projectPath, reloadSignal])

  useEffect(() => {
    const loadAsset = async () => {
      if (!projectPath || !selectedAssetPath) {
        setAsset(null)
        setSelectedNodeId(null)
        setNodeContentDraft('')
        return
      }

      setIsLoadingAsset(true)
      try {
        const loaded = (await readAssetFile(projectPath, selectedAssetPath)) as AssetTree
        setAsset(loaded)

        const firstNode = loaded.root?.children?.[0] || null
        if (firstNode) {
          setSelectedNodeId(firstNode.node_id)
          setNodeContentDraft(firstNode.content || '')
        } else {
          setSelectedNodeId(null)
          setNodeContentDraft('')
        }
      } catch (error) {
        console.error('Failed to load asset detail:', error)
        setAsset(null)
        setSelectedNodeId(null)
        setNodeContentDraft('')
      } finally {
        setIsLoadingAsset(false)
      }
    }

    void loadAsset()
  }, [projectPath, selectedAssetPath])

  const currentDir = useMemo(() => {
    if (!kindRoot) return null
    return findDirByPath(kindRoot, explorerPath) || kindRoot
  }, [explorerPath, kindRoot])

  const explorerEntries = useMemo(
    () => (currentDir ? mapChildrenToExplorerEntries(currentDir.children) : []),
    [currentDir],
  )

  const selectedNode = useMemo(
    () => findAssetNodeById(asset, selectedNodeId),
    [asset, selectedNodeId],
  )

  const nodes = useMemo(() => getAssetDisplayNodes(asset), [asset])

  const handleKindChange = useCallback((nextKind: AssetKind) => {
    setKind(nextKind)
    setExplorerPath(nextKind)
    setSelectedAssetPath(null)
    setCreateDraftKind(null)
    setCreateDraftValue('')
  }, [])

  const handleOpenEntry = useCallback((entry: AssetExplorerEntry) => {
    if (entry.kind === 'dir') {
      setExplorerPath(entry.path)
      return
    }

    setSelectedAssetPath(entry.path)
  }, [])

  const handleGoParent = useCallback(() => {
    const normalizedCurrent = normalizePath(explorerPath)
    if (normalizedCurrent === kind) return

    const parent = parentPath(normalizedCurrent)
    if (!parent || !parent.startsWith(kind)) {
      setExplorerPath(kind)
      return
    }

    setExplorerPath(parent)
  }, [explorerPath, kind])

  const selectNode = useCallback(
    (nodeId: string) => {
      setSelectedNodeId(nodeId)
      const node = findAssetNodeById(asset, nodeId)
      setNodeContentDraft(node?.content || '')
    },
    [asset],
  )

  const saveSelectedNode = useCallback(async () => {
    if (!projectPath || !selectedAssetPath || !asset || !selectedNodeId) return

    try {
      const nextAsset = updateAssetNodeContent(asset, selectedNodeId, nodeContentDraft)
      await saveAssetFile(projectPath, selectedAssetPath, nextAsset)
      setAsset(nextAsset)
      addToast({ title: '保存成功', description: '知识库已更新', variant: 'success' })
    } catch (error) {
      console.error('Failed to save asset:', error)
      addToast({ title: '保存失败', description: String(error), variant: 'destructive' })
    }
  }, [addToast, asset, nodeContentDraft, projectPath, selectedAssetPath, selectedNodeId])

  const cancelCreateDraft = useCallback(() => {
    setCreateDraftKind(null)
    setCreateDraftValue('')
  }, [])

  const startCreateFolder = useCallback(() => {
    setCreateDraftKind('folder')
    setCreateDraftValue('')
  }, [])

  const startCreateFile = useCallback(() => {
    setCreateDraftKind('file')
    setCreateDraftValue('')
  }, [])

  const confirmCreateDraft = useCallback(async () => {
    if (!projectPath || !createDraftKind) return

    const name = createDraftValue.trim()
    if (!name) {
      cancelCreateDraft()
      return
    }

    const duplicated = explorerEntries.some((entry) => entry.kind === createDraftKind && entry.title.trim() === name)
    if (duplicated) {
      addToast({ title: '创建失败', description: '当前目录下已存在同名项', variant: 'warning' })
      return
    }

    try {
      if (createDraftKind === 'folder') {
        await createAssetFolder(projectPath, explorerPath, name)
        addToast({ title: '创建成功', description: `已新建文件夹：${name}`, variant: 'success' })
      } else {
        const path = await createAssetFile(projectPath, explorerPath, kind, name)
        const normalized = normalizePath(path)
        setSelectedAssetPath(normalized)
        addToast({ title: '创建成功', description: `已新建${ASSET_KIND_LABELS[kind]}：${name}`, variant: 'success' })
      }

      setReloadSignal((prev) => prev + 1)
      cancelCreateDraft()
    } catch (error) {
      console.error('Failed to create knowledge item:', error)
      addToast({ title: '创建失败', description: String(error), variant: 'destructive' })
    }
  }, [
    addToast,
    cancelCreateDraft,
    createDraftKind,
    createDraftValue,
    explorerEntries,
    explorerPath,
    kind,
    projectPath,
  ])

  const isLoading = isLoadingTree || isLoadingAsset

  return (
    <AssetManagerView
      projectPath={projectPath}
      kind={kind}
      currentFolderLabel={currentDir ? getNodeTitle(currentDir) : '根目录'}
      canGoParent={normalizePath(explorerPath) !== kind}
      isLoading={isLoading}
      explorerEntries={explorerEntries}
      selectedAssetPath={selectedAssetPath}
      asset={asset}
      nodes={nodes}
      selectedNode={selectedNode}
      selectedNodeId={selectedNodeId}
      nodeContentDraft={nodeContentDraft}
      createDraftKind={createDraftKind}
      createDraftValue={createDraftValue}
      onKindChange={handleKindChange}
      onOpenEntry={handleOpenEntry}
      onGoParent={handleGoParent}
      onNodeSelect={selectNode}
      onNodeDraftChange={setNodeContentDraft}
      onCreateDraftValueChange={setCreateDraftValue}
      onConfirmCreateDraft={confirmCreateDraft}
      onCancelCreateDraft={cancelCreateDraft}
      onStartCreateFile={startCreateFile}
      onStartCreateFolder={startCreateFolder}
      onSaveSelectedNode={saveSelectedNode}
    />
  )
}

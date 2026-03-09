import { useEffect } from 'react'

import {
  getMagicAssetsTree,
  readMagicAsset,
  type AssetKind,
  type MagicAssetNode,
} from '@/features/assets-management'

import { flattenAssetNodes, type AssetTree } from './asset-tree-utils'

export type AssetFileSummary = {
  path: string
  title: string
  assetId?: string
  modified_at?: number
  parentPath: string
}

export type AssetFolderSummary = {
  path: string
  name: string
  title?: string
  parentPath: string
  level: number
}

type LoadAssetListInput = {
  projectPath: string | null
  kind: AssetKind
  reloadSignal: number
  setIsLoading: (value: boolean) => void
  setAssets: (assets: AssetFileSummary[]) => void
  setSelectedAssetPath: (path: string | null | ((prev: string | null) => string | null)) => void
}

type LoadAssetDetailInput = {
  projectPath: string | null
  selectedAssetPath: string | null
  setIsLoading: (value: boolean) => void
  setAsset: (asset: AssetTree | null) => void
  setSelectedNodeId: (id: string | null) => void
  setNodeContentDraft: (text: string) => void
}

type LoadAssetFoldersInput = {
  projectPath: string | null
  kind: AssetKind
  reloadSignal: number
  setFolders: (folders: AssetFolderSummary[]) => void
}

function normalizePath(value: string) {
  return String(value || '').replace(/\\/g, '/')
}

function parentPathOf(path: string) {
  const normalized = normalizePath(path)
  const parts = normalized.split('/')
  parts.pop()
  return parts.join('/')
}

function isDirNode(node: MagicAssetNode): node is Extract<MagicAssetNode, { kind: 'dir' }> {
  return node.kind === 'dir'
}

function isFileNode(node: MagicAssetNode): node is Extract<MagicAssetNode, { kind: 'file' }> {
  return node.kind === 'file'
}

function findKindRoot(nodes: MagicAssetNode[], kind: AssetKind) {
  return nodes.find(
    (node): node is Extract<MagicAssetNode, { kind: 'dir' }> =>
      isDirNode(node) && normalizePath(node.path) === kind,
  )
}

function collectKindFiles(nodes: MagicAssetNode[], kind: AssetKind): AssetFileSummary[] {
  const kindRoot = findKindRoot(nodes, kind)
  if (!kindRoot) return []

  const files: AssetFileSummary[] = []

  const walk = (dir: Extract<MagicAssetNode, { kind: 'dir' }>) => {
    for (const child of dir.children) {
      if (isDirNode(child)) {
        walk(child)
        continue
      }

      if (!isFileNode(child)) continue

      files.push({
        path: normalizePath(child.path),
        title: child.title || child.name,
        assetId: child.asset_id,
        modified_at: child.modified_at,
        parentPath: parentPathOf(child.path),
      })
    }
  }

  walk(kindRoot)
  return files
}

function collectKindFolders(nodes: MagicAssetNode[], kind: AssetKind): AssetFolderSummary[] {
  const kindRoot = findKindRoot(nodes, kind)
  if (!kindRoot) return []

  const folders: AssetFolderSummary[] = []

  const walk = (dir: Extract<MagicAssetNode, { kind: 'dir' }>, level: number) => {
    for (const child of dir.children) {
      if (!isDirNode(child)) continue

      folders.push({
        path: normalizePath(child.path),
        name: child.name,
        title: child.title,
        parentPath: normalizePath(dir.path),
        level,
      })

      walk(child, level + 1)
    }
  }

  walk(kindRoot, 0)
  return folders
}

export function useLoadAssetList({
  projectPath,
  kind,
  reloadSignal,
  setIsLoading,
  setAssets,
  setSelectedAssetPath,
}: LoadAssetListInput) {
  useEffect(() => {
    const load = async () => {
      if (!projectPath) {
        setAssets([])
        setSelectedAssetPath(null)
        return
      }

      setIsLoading(true)
      try {
        const tree = await getMagicAssetsTree(projectPath)
        const files = collectKindFiles(tree, kind)
        setAssets(files)

        setSelectedAssetPath((prev) => {
          if (prev && files.some((item) => item.path === prev)) return prev
          return files[0]?.path || null
        })
      } catch (error) {
        console.error('Failed to load assets:', error)
      } finally {
        setIsLoading(false)
      }
    }

    void load()
  }, [kind, projectPath, reloadSignal, setAssets, setIsLoading, setSelectedAssetPath])
}

export function useLoadAssetDetail({
  projectPath,
  selectedAssetPath,
  setIsLoading,
  setAsset,
  setSelectedNodeId,
  setNodeContentDraft,
}: LoadAssetDetailInput) {
  useEffect(() => {
    const load = async () => {
      if (!projectPath || !selectedAssetPath) {
        setAsset(null)
        setSelectedNodeId(null)
        setNodeContentDraft('')
        return
      }

      setIsLoading(true)
      try {
        const asset = (await readMagicAsset(projectPath, selectedAssetPath)) as AssetTree
        setAsset(asset)

        const firstNode = flattenAssetNodes(asset.root).find((node) => node.level > 0) || null
        setSelectedNodeId(firstNode?.node_id || null)
        setNodeContentDraft(firstNode?.content || '')
      } catch (error) {
        console.error('Failed to load asset detail:', error)
      } finally {
        setIsLoading(false)
      }
    }

    void load()
  }, [projectPath, selectedAssetPath, setAsset, setIsLoading, setNodeContentDraft, setSelectedNodeId])
}

export function useLoadAssetFolders({
  projectPath,
  kind,
  reloadSignal,
  setFolders,
}: LoadAssetFoldersInput) {
  useEffect(() => {
    const load = async () => {
      if (!projectPath) {
        setFolders([])
        return
      }

      try {
        const nodes = await getMagicAssetsTree(projectPath)
        setFolders(collectKindFolders(nodes, kind))
      } catch (error) {
        console.error('Failed to load asset folders:', error)
        setFolders([])
      }
    }

    void load()
  }, [kind, projectPath, reloadSignal, setFolders])
}

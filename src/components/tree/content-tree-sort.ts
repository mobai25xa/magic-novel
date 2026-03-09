import type { FileNode } from './content-tree-types'

type TocSort = {
  field: 'manual' | 'name' | 'createdAt' | 'updatedAt'
  order: 'asc' | 'desc'
}

function compareBySortField(a: FileNode, b: FileNode, tocSort: TocSort): number {
  const orderFactor = tocSort.order === 'asc' ? 1 : -1
  const label = (node: FileNode) => (node.title || node.name || '').toString()

  if (tocSort.field === 'name') {
    const result = label(a).localeCompare(label(b), 'zh')
    if (result !== 0) return result * orderFactor
  }

  if (tocSort.field === 'createdAt') {
    const result = (a.createdAt ?? 0) - (b.createdAt ?? 0)
    if (result !== 0) return result * orderFactor
  }

  if (tocSort.field === 'updatedAt') {
    const result = (a.updatedAt ?? 0) - (b.updatedAt ?? 0)
    if (result !== 0) return result * orderFactor
  }

  return a.path.localeCompare(b.path, 'en')
}

function compareNodes(a: FileNode, b: FileNode, tocSort: TocSort): number {
  if (a.kind === 'knowledge' && b.kind !== 'knowledge') return -1
  if (a.kind !== 'knowledge' && b.kind === 'knowledge') return 1

  const aIsDir = a.kind === 'dir' || a.kind === 'asset_dir'
  const bIsDir = b.kind === 'dir' || b.kind === 'asset_dir'
  if (aIsDir && !bIsDir) return -1
  if (!aIsDir && bIsDir) return 1

  return compareBySortField(a, b, tocSort)
}

export function sortTree(nodes: FileNode[], tocSort: TocSort): FileNode[] {
  const sortRecursively = (sourceNodes: FileNode[]): FileNode[] => {
    const cloned = sourceNodes.map((node) => ({
      ...node,
      children: node.children ? sortRecursively(node.children) : node.children,
    }))

    if (tocSort.field !== 'manual') {
      cloned.sort((a, b) => compareNodes(a, b, tocSort))
    }

    return cloned
  }

  return sortRecursively(nodes)
}

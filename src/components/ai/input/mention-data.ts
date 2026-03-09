import { useProjectStore } from '@/state/project'

export type MentionItem = {
  id: string
  type: 'chapter' | 'volume' | 'character' | 'location' | 'asset'
  label: string
  path: string
  group: string
}

type FileNode = {
  kind: 'dir' | 'chapter' | 'knowledge' | 'asset_dir' | 'asset_file'
  name: string
  path: string
  children?: FileNode[]
  chapterId?: string
  title?: string
  assetRelativePath?: string
}

function collectMentionItems(nodes: FileNode[], parentVolume?: string): MentionItem[] {
  const items: MentionItem[] = []

  for (const node of nodes) {
    if (node.kind === 'dir') {
      // Volume
      items.push({
        id: node.path,
        type: 'volume',
        label: node.title ?? node.name,
        path: node.path,
        group: 'volume',
      })
      if (node.children) {
        items.push(...collectMentionItems(node.children, node.title ?? node.name))
      }
    } else if (node.kind === 'chapter') {
      items.push({
        id: node.chapterId ?? node.path,
        type: 'chapter',
        label: node.title ?? node.name,
        path: node.path,
        group: 'chapter',
      })
    } else if (node.kind === 'asset_dir') {
      if (node.children) {
        items.push(...collectMentionItems(node.children, parentVolume))
      }
    } else if (node.kind === 'asset_file') {
      const assetType = guessAssetType(node)
      items.push({
        id: node.assetRelativePath ?? node.path,
        type: assetType,
        label: node.title ?? node.name,
        path: node.assetRelativePath ?? node.path,
        group: assetType,
      })
    }
  }

  return items
}

function guessAssetType(node: FileNode): 'character' | 'location' | 'asset' {
  const lower = (node.title ?? node.name).toLowerCase()
  const pathLower = node.path.toLowerCase()

  if (pathLower.includes('character') || pathLower.includes('角色')) return 'character'
  if (pathLower.includes('location') || pathLower.includes('地点')) return 'location'
  if (lower.includes('角色') || lower.includes('人物')) return 'character'
  if (lower.includes('地点') || lower.includes('场景')) return 'location'

  return 'asset'
}

function matchesQuery(label: string, query: string): boolean {
  if (!query) return true
  const lowerLabel = label.toLowerCase()
  const lowerQuery = query.toLowerCase()
  return lowerLabel.includes(lowerQuery)
}

export function getMentionItems(query: string): MentionItem[] {
  const tree = useProjectStore.getState().tree
  const allItems = collectMentionItems(tree)
  return allItems.filter((item) => matchesQuery(item.label, query))
}

const GROUP_ORDER: Record<string, number> = {
  chapter: 0,
  volume: 1,
  character: 2,
  location: 3,
  asset: 4,
}

export function groupMentionItems(items: MentionItem[]): { group: string; items: MentionItem[] }[] {
  const groups = new Map<string, MentionItem[]>()

  for (const item of items) {
    const existing = groups.get(item.group)
    if (existing) {
      existing.push(item)
    } else {
      groups.set(item.group, [item])
    }
  }

  return Array.from(groups.entries())
    .sort(([a], [b]) => (GROUP_ORDER[a] ?? 99) - (GROUP_ORDER[b] ?? 99))
    .map(([group, groupItems]) => ({ group, items: groupItems }))
}

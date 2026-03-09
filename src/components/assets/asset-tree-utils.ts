export type AssetNode = {
  node_id: string
  title: string
  level: number
  content: string
  children: AssetNode[]
}

export type AssetTree = {
  schema_version: number
  id: string
  kind: string
  title: string
  root: AssetNode
}

export function flattenAssetNodes(node: AssetNode): AssetNode[] {
  const output: AssetNode[] = []

  const walk = (current: AssetNode) => {
    output.push(current)
    current.children?.forEach(walk)
  }

  walk(node)
  return output
}

export function getAssetDisplayNodes(asset: AssetTree | null) {
  if (!asset) return []
  return flattenAssetNodes(asset.root).filter((node) => node.level > 0)
}

export function findAssetNodeById(asset: AssetTree | null, nodeId: string | null) {
  if (!asset || !nodeId) return null
  return flattenAssetNodes(asset.root).find((node) => node.node_id === nodeId) || null
}

function updateNodeContent(node: AssetNode, nodeId: string, content: string): [AssetNode, boolean] {
  let hasChildChange = false
  const children = node.children.map((child) => {
    const [nextChild, changed] = updateNodeContent(child, nodeId, content)
    hasChildChange = hasChildChange || changed
    return nextChild
  })

  if (node.node_id === nodeId) {
    if (node.content === content && !hasChildChange) {
      return [node, false]
    }

    return [{ ...node, content, children }, true]
  }

  if (hasChildChange) {
    return [{ ...node, children }, true]
  }

  return [node, false]
}

export function updateAssetNodeContent(asset: AssetTree, nodeId: string, content: string): AssetTree {
  const [root, changed] = updateNodeContent(asset.root, nodeId, content)
  if (!changed) return asset
  return { ...asset, root }
}

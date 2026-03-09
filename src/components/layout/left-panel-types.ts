export type LeftPanelFileNode = {
  kind: 'dir' | 'chapter' | 'knowledge' | 'asset_dir' | 'asset_file'
  name: string
  path: string
  children?: LeftPanelFileNode[]
  chapter_id?: string
  title?: string
  text_length_no_whitespace?: number
  word_count?: number
  status?: string
  created_at?: number
  updated_at?: number
  assetRelativePath?: string
}

function normalizeAssetRelativePath(path: string, kind: LeftPanelFileNode['kind']) {
  if (kind !== 'asset_dir' && kind !== 'asset_file') return undefined
  const normalized = String(path || '').replace(/\\/g, '/')
  return normalized.startsWith('magic_assets/') ? normalized.slice('magic_assets/'.length) : normalized
}

export function convertFileNode(node: LeftPanelFileNode) {
  return {
    kind: node.kind,
    name: node.name,
    path: node.path,
    children: node.children?.map(convertFileNode),
    chapterId: node.chapter_id,
    title: node.title,
    textLengthNoWhitespace: node.word_count ?? node.text_length_no_whitespace,
    status: node.status,
    createdAt: node.created_at,
    updatedAt: node.updated_at,
    assetRelativePath: node.assetRelativePath || normalizeAssetRelativePath(node.path, node.kind),
  }
}

import { v4 as uuidv4 } from 'uuid'

type TiptapNode = {
  type?: string
  attrs?: Record<string, unknown>
  text?: string
  content?: TiptapNode[]
}

export type KnowledgeAssetNode = {
  node_id: string
  title: string
  level: number
  content: string
  children: KnowledgeAssetNode[]
  tags?: string[] | null
}

export type KnowledgeAssetTree = {
  schema_version: number
  id: string
  kind: string
  title: string
  source?: unknown
  root: KnowledgeAssetNode
}

const DOCUMENT_CONTENT_PREFIX = '__magic_novel_doc_json__:'

const EMPTY_EDITOR_DOC: TiptapNode = {
  type: 'doc',
  content: [{ type: 'paragraph', content: [] }],
}

function asNodeArray(value: unknown): TiptapNode[] {
  return Array.isArray(value) ? (value as TiptapNode[]) : []
}

function asAssetChildren(value: unknown): KnowledgeAssetNode[] {
  return Array.isArray(value) ? (value as KnowledgeAssetNode[]) : []
}

function asAssetTree(value: unknown): KnowledgeAssetTree | null {
  if (!value || typeof value !== 'object') return null
  const maybe = value as Partial<KnowledgeAssetTree>
  if (!maybe.root || typeof maybe.root !== 'object') return null
  return maybe as KnowledgeAssetTree
}

function createFallbackRoot(): KnowledgeAssetNode {
  return {
    node_id: uuidv4(),
    title: 'root',
    level: 0,
    content: '',
    children: [],
  }
}

function normalizeHeadingLevel(level: number) {
  if (!Number.isFinite(level)) return 1
  return Math.max(1, Math.min(3, Math.floor(level)))
}

function paragraphNode(text: string): TiptapNode {
  return text
    ? { type: 'paragraph', content: [{ type: 'text', text }] }
    : { type: 'paragraph', content: [] }
}

function headingNode(level: number, text: string): TiptapNode {
  return {
    type: 'heading',
    attrs: { level: normalizeHeadingLevel(level) },
    content: [{ type: 'text', text }],
  }
}

function appendParagraphsFromText(text: string, out: TiptapNode[]) {
  const normalized = String(text || '').replace(/\r\n/g, '\n').trim()
  if (!normalized) return

  const chunks = normalized
    .split(/\n{2,}/)
    .map((item) => item.trim())
    .filter(Boolean)

  if (chunks.length === 0) {
    out.push(paragraphNode(''))
    return
  }

  for (const chunk of chunks) {
    out.push(paragraphNode(chunk))
  }
}

function collectLegacyTreeBlocks(node: KnowledgeAssetNode, out: TiptapNode[]) {
  const title = String(node.title || '').trim()
  const level = normalizeHeadingLevel(Number(node.level || 1))

  if (title) {
    out.push(headingNode(level, title))
  }

  appendParagraphsFromText(String(node.content || ''), out)

  for (const child of asAssetChildren(node.children)) {
    collectLegacyTreeBlocks(child, out)
  }
}

function collectLegacyRootAsBlocks(root: KnowledgeAssetNode): TiptapNode[] {
  const blocks: TiptapNode[] = []
  appendParagraphsFromText(String(root.content || ''), blocks)

  for (const child of asAssetChildren(root.children)) {
    collectLegacyTreeBlocks(child, blocks)
  }

  return blocks
}

function isEditorDoc(value: unknown): value is TiptapNode {
  if (!value || typeof value !== 'object') return false
  const maybe = value as TiptapNode
  return maybe.type === 'doc' && Array.isArray(maybe.content)
}

function normalizeEditorDoc(value: unknown): TiptapNode {
  if (!isEditorDoc(value)) {
    return EMPTY_EDITOR_DOC
  }

  const blocks = asNodeArray(value.content)
  if (blocks.length === 0) {
    return EMPTY_EDITOR_DOC
  }

  return {
    type: 'doc',
    content: blocks,
  }
}

function decodeEditorDocFromContent(content: string): TiptapNode | null {
  const raw = String(content || '').trim()
  if (!raw) return null

  const decodeRaw = (jsonRaw: string): TiptapNode | null => {
    try {
      const parsed = JSON.parse(jsonRaw)
      return isEditorDoc(parsed) ? normalizeEditorDoc(parsed) : null
    } catch {
      return null
    }
  }

  if (raw.startsWith(DOCUMENT_CONTENT_PREFIX)) {
    return decodeRaw(raw.slice(DOCUMENT_CONTENT_PREFIX.length))
  }

  return decodeRaw(raw)
}

function encodeEditorDocToContent(editorDoc: TiptapNode): string {
  return `${DOCUMENT_CONTENT_PREFIX}${JSON.stringify(editorDoc)}`
}

function findDocumentNode(root: KnowledgeAssetNode): KnowledgeAssetNode | null {
  const stack: KnowledgeAssetNode[] = [...asAssetChildren(root.children)]

  while (stack.length > 0) {
    const node = stack.shift()!
    if (decodeEditorDocFromContent(String(node.content || ''))) {
      return node
    }

    stack.push(...asAssetChildren(node.children))
  }

  return null
}

function findLegacyPrimaryNode(root: KnowledgeAssetNode): KnowledgeAssetNode | null {
  const directChildren = asAssetChildren(root.children)
  if (directChildren.length === 0) return null

  const firstLevelOne = directChildren.find((item) => normalizeHeadingLevel(Number(item.level || 1)) === 1)
  return firstLevelOne || directChildren[0]
}

export function assetTreeToEditorDoc(asset: unknown): TiptapNode {
  const parsed = asAssetTree(asset)
  if (!parsed) return EMPTY_EDITOR_DOC

  const root = parsed.root && typeof parsed.root === 'object' ? parsed.root : createFallbackRoot()

  const nodeWithDoc = findDocumentNode(root)
  if (nodeWithDoc) {
    const decoded = decodeEditorDocFromContent(String(nodeWithDoc.content || ''))
    if (decoded) return decoded
  }

  const rootDoc = decodeEditorDocFromContent(String(root.content || ''))
  if (rootDoc) return rootDoc

  const legacyBlocks = collectLegacyRootAsBlocks(root)
  if (legacyBlocks.length === 0) {
    return EMPTY_EDITOR_DOC
  }

  return {
    type: 'doc',
    content: legacyBlocks,
  }
}

function buildDocumentNode(input: {
  existingNode: KnowledgeAssetNode | null
  legacyFallbackNode: KnowledgeAssetNode | null
  assetTitle: string
  editorDoc: TiptapNode
}): KnowledgeAssetNode {
  const title = String(input.assetTitle || '').trim() || '内容'

  const fallbackNodeId =
    input.legacyFallbackNode &&
    typeof input.legacyFallbackNode.node_id === 'string' &&
    input.legacyFallbackNode.node_id.trim().length > 0
      ? input.legacyFallbackNode.node_id
      : null

  return {
    node_id:
      input.existingNode && typeof input.existingNode.node_id === 'string' && input.existingNode.node_id.trim().length > 0
        ? input.existingNode.node_id
        : fallbackNodeId || uuidv4(),
    title,
    level: 1,
    content: encodeEditorDocToContent(input.editorDoc),
    children: [],
    tags: input.existingNode?.tags ?? input.legacyFallbackNode?.tags,
  }
}

export function editorDocToAssetTree(asset: unknown, editorDoc: unknown): KnowledgeAssetTree {
  const parsedAsset = asAssetTree(asset)
  const base: KnowledgeAssetTree = parsedAsset || {
    schema_version: 1,
    id: uuidv4(),
    kind: 'lore',
    title: '未命名知识库',
    root: createFallbackRoot(),
  }

  const nextRootBase = base.root && typeof base.root === 'object' ? base.root : createFallbackRoot()
  const nextDoc = normalizeEditorDoc(editorDoc)
  const existingDocumentNode = findDocumentNode(nextRootBase)
  const legacyFallbackNode = existingDocumentNode ? null : findLegacyPrimaryNode(nextRootBase)
  const documentNode = buildDocumentNode({
    existingNode: existingDocumentNode,
    legacyFallbackNode,
    assetTitle: base.title,
    editorDoc: nextDoc,
  })

  return {
    ...base,
    root: {
      ...nextRootBase,
      node_id:
        typeof nextRootBase.node_id === 'string' && nextRootBase.node_id.trim().length > 0
          ? nextRootBase.node_id
          : uuidv4(),
      title: 'root',
      level: 0,
      content: '',
      children: [documentNode],
    },
  }
}

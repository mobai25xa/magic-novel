import { readAssetFile } from '@/features/assets-management'
import { assetTreeToEditorDoc, type KnowledgeAssetTree } from '@/features/assets-management/asset-editor-document'
import { readChapter } from '@/features/editor-reading'
import { readKnowledgeDocument } from '@/features/knowledge-documents'
import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useLayoutStore } from '@/stores/layout-store'
import { useEditorUiStore } from '@/stores/editor-ui-store'

export type EditorTargetKind = 'chapter' | 'asset' | 'knowledge'

export type ParsedEditorTargetRef =
  | { kind: 'chapter'; chapterPath: string }
  | { kind: 'asset'; assetPath: string }
  | { kind: 'knowledge'; knowledgePath: string }

function normalizeSlashPath(path: string) {
  return path.trim().replace(/\\/g, '/')
}

function containsParentTraversal(path: string) {
  return path.split('/').some((segment) => segment === '..')
}

export function normalizeChapterPath(rawPath: string) {
  const normalized = normalizeSlashPath(rawPath)
    .replace(/^chapter:/, '')
    .replace(/^manuscripts\//, '')
    .replace(/^\/+/, '')
  return normalized
}

export function normalizeAssetPath(rawPath: string) {
  const normalized = normalizeSlashPath(rawPath)
    .replace(/^asset:/, '')
    .replace(/^assets\//, '')
    .replace(/^\/+/, '')
  return normalized
}

export function normalizeKnowledgePath(rawPath: string) {
  const normalized = normalizeSlashPath(rawPath)
    .replace(/^knowledge:/, '')
    .replace(/^\/+/, '')

  if (!normalized) {
    return '.magic_novel'
  }

  if (normalized === '.magic_novel' || normalized.startsWith('.magic_novel/')) {
    return normalized
  }

  return `.magic_novel/${normalized}`
}

export function parseEditorTargetRef(rawRef: string): ParsedEditorTargetRef | null {
  const raw = normalizeSlashPath(rawRef)
  if (!raw) return null

  if (raw.startsWith('chapter:')) {
    const chapterPath = normalizeChapterPath(raw)
    if (!chapterPath || containsParentTraversal(chapterPath)) return null
    return { kind: 'chapter', chapterPath }
  }

  if (raw.startsWith('asset:')) {
    const assetPath = normalizeAssetPath(raw)
    if (!assetPath || containsParentTraversal(assetPath)) return null
    return { kind: 'asset', assetPath }
  }

  if (raw.startsWith('knowledge:')) {
    const knowledgePath = normalizeKnowledgePath(raw)
    if (!knowledgePath || containsParentTraversal(knowledgePath)) return null
    return { kind: 'knowledge', knowledgePath }
  }

  if (raw.startsWith('manuscripts/')) {
    const chapterPath = normalizeChapterPath(raw)
    if (!chapterPath || containsParentTraversal(chapterPath)) return null
    return { kind: 'chapter', chapterPath }
  }

  if (raw.startsWith('.magic_novel/')) {
    const knowledgePath = normalizeKnowledgePath(raw)
    if (!knowledgePath || containsParentTraversal(knowledgePath)) return null
    return { kind: 'knowledge', knowledgePath }
  }

  if (raw.startsWith('assets/')) {
    const assetPath = normalizeAssetPath(raw)
    if (!assetPath || containsParentTraversal(assetPath)) return null
    return { kind: 'asset', assetPath }
  }

  return null
}

async function maybeAutoSaveBeforeSwitch() {
  const editorStore = useEditorStore.getState()
  if (!editorStore.isDirty) return

  try {
    const maybeWindow = window as Window & { __manualSave?: () => Promise<void> }
    const manualSave = maybeWindow.__manualSave
    if (manualSave) await manualSave()
  } catch (error) {
    console.error('[editor-navigation] Failed to auto-save before switch:', error)
  }
}

function resolveAssetTitle(asset: KnowledgeAssetTree | Record<string, unknown> | null) {
  if (!asset) {
    return null
  }

  if (typeof asset !== 'object') {
    return null
  }

  const title = 'title' in asset ? (asset as { title?: unknown }).title : undefined
  return typeof title === 'string' && title.trim() ? title.trim() : null
}

export async function openEditorTarget(
  ref: string,
  options?: {
    revealLeftTree?: boolean
    switchLeftTab?: boolean
  },
): Promise<boolean> {
  const parsed = parseEditorTargetRef(ref)
  if (!parsed) {
    return false
  }

  const projectPath = useProjectStore.getState().projectPath
  if (!projectPath) {
    return false
  }

  if (options?.revealLeftTree) {
    useLayoutStore.setState({ isLeftPanelVisible: true })
  }

  if (options?.switchLeftTab) {
    useEditorUiStore.getState().setLeftPanelTab(parsed.kind === 'chapter' ? 'outline' : 'knowledge')
  }

  await maybeAutoSaveBeforeSwitch()

  try {
    if (parsed.kind === 'chapter') {
      const editorStore = useEditorStore.getState()
      const chapterPath = parsed.chapterPath

      if (editorStore.currentDocKind === 'chapter' && editorStore.currentChapterPath === chapterPath) {
        useProjectStore.getState().setSelectedPath(chapterPath)
        useAgentChatStore.getState().setActiveChapterPath(chapterPath)
        return true
      }

      const chapter = await readChapter(projectPath, chapterPath)
      useProjectStore.getState().setSelectedPath(chapterPath)
      editorStore.setCurrentChapter(chapter.id, chapterPath, chapter.title)
      editorStore.setContent(chapter.content)
      editorStore.setIsDirty(false)
      editorStore.setLastOpened(projectPath, chapterPath, chapter.id, chapter.title)
      useAgentChatStore.getState().setActiveChapterPath(chapterPath)
      return true
    }

    const editorStore = useEditorStore.getState()

    if (parsed.kind === 'knowledge') {
      const knowledgePath = parsed.knowledgePath
      if (editorStore.currentDocKind === 'knowledge' && editorStore.currentAssetPath === knowledgePath) {
        useProjectStore.getState().setSelectedPath(`knowledge:${knowledgePath}`)
        return true
      }

      const document = await readKnowledgeDocument(projectPath, knowledgePath)
      useProjectStore.getState().setSelectedPath(`knowledge:${knowledgePath}`)
      editorStore.setCurrentKnowledge(knowledgePath, document.title || null)
      editorStore.setContent(document.content)
      editorStore.setIsDirty(false)
      return true
    }

    const assetPath = parsed.assetPath
    if (editorStore.currentDocKind === 'asset' && editorStore.currentAssetPath === assetPath) {
      useProjectStore.getState().setSelectedPath(`assets/${assetPath}`)
      return true
    }

    const asset = (await readAssetFile(projectPath, assetPath)) as KnowledgeAssetTree
    const title = resolveAssetTitle(asset as unknown as KnowledgeAssetTree)
    const content = assetTreeToEditorDoc(asset)

    useProjectStore.getState().setSelectedPath(`assets/${assetPath}`)
    editorStore.setCurrentAsset(assetPath, title)
    editorStore.setContent(content)
    editorStore.setIsDirty(false)
    return true
  } catch (error) {
    console.error('[editor-navigation] Failed to open target:', error)
    return false
  }
}

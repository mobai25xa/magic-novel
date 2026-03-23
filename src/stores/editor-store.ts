import { create } from 'zustand'
import type { Editor } from '@tiptap/react'

interface PatchOp {
  op: 'insert_blocks' | 'update_block' | 'delete_blocks'
  after_block_id?: string | null
  blocks?: unknown[]
  block_id?: string
  before?: unknown
  after?: unknown
  block_ids?: string[]
}

interface EditorState {
  editor: Editor | null
  currentDocKind: 'chapter' | 'asset' | 'knowledge' | null

  currentChapterId: string | null
  currentChapterPath: string | null
  currentChapterTitle: string | null

  currentAssetPath: string | null
  currentAssetTitle: string | null

  content: unknown
  isDirty: boolean
  isSaving: boolean
  lastSavedAt: number | null

  pendingExternalChapterRefresh: { projectPath: string; chapterPath: string; requestedAt: number } | null

  // === Beta: Last Opened State ===
  lastOpenedProjectPath: string | null
  lastOpenedChapterPath: string | null
  lastOpenedChapterId: string | null
  lastOpenedChapterTitle: string | null

  setEditor: (editor: Editor | null) => void
  setCurrentChapter: (id: string | null, path: string | null, title?: string | null) => void
  setCurrentAsset: (relativePath: string | null, title?: string | null) => void
  setCurrentKnowledge: (virtualPath: string | null, title?: string | null) => void
  setContent: (content: unknown) => void
  setIsDirty: (dirty: boolean) => void
  setIsSaving: (saving: boolean) => void
  setLastSavedAt: (timestamp: number | null) => void
  setPendingExternalChapterRefresh: (value: EditorState['pendingExternalChapterRefresh']) => void
  clearPendingExternalChapterRefresh: () => void
  setLastOpened: (projectPath: string, chapterPath: string, chapterId: string, chapterTitle: string | null) => void
  applyPatch: (patch: PatchOp[]) => void
  reset: () => void
}

// === Beta: Load persisted lastOpened from localStorage ===
function loadLastOpened() {
  try {
    const raw = localStorage.getItem('magic-novel-last-opened')
    if (raw) return JSON.parse(raw)
  } catch {
    // ignore
  }
  return { projectPath: null, chapterPath: null, chapterId: null, chapterTitle: null }
}
const _lastOpened = loadLastOpened()

export const useEditorStore = create<EditorState>((set, get) => ({
  editor: null,
  currentDocKind: null,

  currentChapterId: null,
  currentChapterPath: null,
  currentChapterTitle: null,

  currentAssetPath: null,
  currentAssetTitle: null,

  content: null,
  isDirty: false,
  isSaving: false,
  lastSavedAt: null,
  pendingExternalChapterRefresh: null,

  // === Beta: Last Opened State ===
  lastOpenedProjectPath: _lastOpened.projectPath,
  lastOpenedChapterPath: _lastOpened.chapterPath,
  lastOpenedChapterId: _lastOpened.chapterId,
  lastOpenedChapterTitle: _lastOpened.chapterTitle,

  setEditor: (editor) => set({ editor }),

  setCurrentChapter: (id, path, title = null) =>
    set({
      currentDocKind: id && path ? 'chapter' : null,
      currentChapterId: id,
      currentChapterPath: path,
      currentChapterTitle: title,
      currentAssetPath: null,
      currentAssetTitle: null,
    }),

  setCurrentAsset: (relativePath, title = null) =>
    set({
      currentDocKind: relativePath ? 'asset' : null,
      currentAssetPath: relativePath,
      currentAssetTitle: title,
      currentChapterId: null,
      currentChapterPath: null,
      currentChapterTitle: null,
    }),

  setCurrentKnowledge: (virtualPath, title = null) =>
    set({
      currentDocKind: virtualPath ? 'knowledge' : null,
      currentAssetPath: virtualPath,
      currentAssetTitle: title,
      currentChapterId: null,
      currentChapterPath: null,
      currentChapterTitle: null,
    }),

  setContent: (content) => set({ content, isDirty: true }),
  setIsDirty: (isDirty) => set({ isDirty }),
  setIsSaving: (isSaving) => set({ isSaving }),
  setLastSavedAt: (lastSavedAt) => set({ lastSavedAt }),
  setPendingExternalChapterRefresh: (pendingExternalChapterRefresh) => set({ pendingExternalChapterRefresh }),
  clearPendingExternalChapterRefresh: () => set({ pendingExternalChapterRefresh: null }),
  setLastOpened: (projectPath, chapterPath, chapterId, chapterTitle) => {
    const data = { projectPath, chapterPath, chapterId, chapterTitle }
    set({
      lastOpenedProjectPath: projectPath,
      lastOpenedChapterPath: chapterPath,
      lastOpenedChapterId: chapterId,
      lastOpenedChapterTitle: chapterTitle,
    })
    try {
      localStorage.setItem('magic-novel-last-opened', JSON.stringify(data))
    } catch {
      // ignore
    }
  },
  applyPatch: (patch: PatchOp[]) => {
    const { editor } = get()
    if (!editor) return

    for (const op of patch) {
      switch (op.op) {
        case 'insert_blocks':
          applyInsertBlocks(editor, op.after_block_id ?? null, op.blocks ?? [])
          break
        case 'update_block':
          if (op.block_id && op.after) {
            applyUpdateBlock(editor, op.block_id, op.after)
          }
          break
        case 'delete_blocks':
          if (op.block_ids) {
            applyDeleteBlocks(editor, op.block_ids)
          }
          break
      }
    }

    set({ isDirty: true })
  },
  reset: () =>
    set({
      currentDocKind: null,
      currentChapterId: null,
      currentChapterPath: null,
      currentChapterTitle: null,
      currentAssetPath: null,
      currentAssetTitle: null,
      content: null,
      isDirty: false,
      isSaving: false,
      lastSavedAt: null,
      pendingExternalChapterRefresh: null,
    }),
}))

function applyInsertBlocks(editor: Editor, afterBlockId: string | null, blocks: unknown[]) {
  const { state } = editor
  let insertPos = state.doc.content.size

  if (afterBlockId) {
    state.doc.descendants((node, pos) => {
      if (node.attrs.id === afterBlockId) {
        insertPos = pos + node.nodeSize
        return false
      }
      return true
    })
  }

  editor.chain().focus().insertContentAt(insertPos, blocks as unknown).run()
}

function applyUpdateBlock(editor: Editor, blockId: string, newBlock: unknown) {
  const { state } = editor

  state.doc.descendants((node, pos) => {
    if (node.attrs.id === blockId) {
      const blockWithId = {
        ...(newBlock as Record<string, unknown>),
        attrs: { ...((newBlock as Record<string, unknown>).attrs as Record<string, unknown> || {}), id: blockId }
      }
      editor.chain()
        .focus()
        .setNodeSelection(pos)
        .deleteSelection()
        .insertContentAt(pos, blockWithId as unknown)
        .run()
      return false
    }
    return true
  })
}

function applyDeleteBlocks(editor: Editor, blockIds: string[]) {
  const { state } = editor
  const tr = state.tr
  const positions: { from: number; to: number }[] = []

  state.doc.descendants((node, pos) => {
    if (blockIds.includes(node.attrs.id)) {
      positions.push({ from: pos, to: pos + node.nodeSize })
    }
    return true
  })

  positions.sort((a, b) => b.from - a.from).forEach(({ from, to }) => {
    tr.delete(from, to)
  })

  editor.view.dispatch(tr)
}

import type { Editor } from '@tiptap/react'

type AiBlockOps = {
  getBlockById: (blockId: string) => unknown
  getSelectedBlocks: () => unknown[]
  insertBlocksAfter: (afterBlockId: string | null, blocks: unknown[]) => void
  replaceBlock: (blockId: string, newBlock: unknown) => void
  deleteBlocks: (blockIds: string[]) => void
  highlightBlocks: (blockIds: string[], color: string) => void
  clearAiHighlights: () => void
}

export function getBlockById(editor: Editor | null, blockId: string) {
  if (!editor) return null

  let foundBlock = null
  editor.state.doc.descendants((node) => {
    if (node.attrs.id === blockId) {
      foundBlock = node.toJSON()
      return false
    }
    return true
  })
  return foundBlock
}

export function getSelectedBlocks(editor: Editor | null) {
  if (!editor) return []

  const { from, to } = editor.state.selection
  const blocks: unknown[] = []

  editor.state.doc.nodesBetween(from, to, (node) => {
    if (node.attrs.id) {
      blocks.push(node.toJSON())
    }
    return true
  })

  return blocks
}

export function insertBlocksAfter(editor: Editor | null, afterBlockId: string | null, blocks: unknown[]) {
  if (!editor) return

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

export function replaceBlock(editor: Editor | null, blockId: string, newBlock: unknown) {
  if (!editor) return

  const { state } = editor

  state.doc.descendants((node, pos) => {
    if (node.attrs.id === blockId) {
      const blockWithId = {
        ...(newBlock as Record<string, unknown>),
        attrs: { ...((newBlock as Record<string, unknown>).attrs as Record<string, unknown> || {}), id: blockId }
      }
      editor.chain().focus().setNodeSelection(pos).deleteSelection().insertContentAt(pos, blockWithId as unknown).run()
      return false
    }
    return true
  })
}

export function deleteBlocks(editor: Editor | null, blockIds: string[]) {
  if (!editor) return

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

export function highlightBlocks(editor: Editor | null, blockIds: string[], color: string) {
  if (!editor) return

  blockIds.forEach((blockId) => {
    editor.state.doc.descendants((node, pos) => {
      if (node.attrs.id === blockId && node.isTextblock) {
        const from = pos + 1
        const to = pos + node.nodeSize - 1
        editor.chain().setTextSelection({ from, to }).setHighlight({ color }).run()
        return false
      }
      return true
    })
  })
}

export function clearAiHighlights(editor: Editor | null) {
  if (!editor) return
  editor.chain().selectAll().unsetHighlight().run()
}

export function createAiBlockOps(editor: Editor | null): AiBlockOps {
  return {
    getBlockById: (blockId) => getBlockById(editor, blockId),
    getSelectedBlocks: () => getSelectedBlocks(editor),
    insertBlocksAfter: (afterBlockId, blocks) => insertBlocksAfter(editor, afterBlockId, blocks),
    replaceBlock: (blockId, newBlock) => replaceBlock(editor, blockId, newBlock),
    deleteBlocks: (blockIds) => deleteBlocks(editor, blockIds),
    highlightBlocks: (blockIds, color) => highlightBlocks(editor, blockIds, color),
    clearAiHighlights: () => clearAiHighlights(editor),
  }
}

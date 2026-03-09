import { useCallback } from 'react'
import { useEditorStore } from '@/stores/editor-store'
import { useProjectStore } from '@/stores/project-store'
import { appendAgentHistoryEvent } from '@/features/agent-chat'
import { createAiBlockOps } from './use-ai-agent-block-ops'
import { calculateDocHash, extractPlainTextFromDoc } from './use-ai-agent-helpers'
import type { AgentEditorState } from '@/agent/types'

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text
  return text.slice(0, maxLen) + '...'
}

export function useAiAgent() {
  const { editor, currentChapterId } = useEditorStore()
  const { projectPath } = useProjectStore()

  const getCurrentChapterContent = useCallback(() => {
    if (!editor) return null
    return editor.getJSON()
  }, [editor])

  const blockOps = createAiBlockOps(editor)

  const getCurrentWordCount = useCallback(() => {
    if (!editor) return 0

    const json = editor.getJSON()
    const text = extractPlainTextFromDoc(json)
    return text.replace(/\s/g, '').length
  }, [editor])

  const getEditorState = useCallback((): AgentEditorState | null => {
    if (!editor) return null

    const { from, to } = editor.state.selection
    const doc = editor.state.doc

    // Selected text
    const selectedText = from !== to
      ? truncate(doc.textBetween(from, to, '\n'), 800)
      : undefined

    // Cursor paragraph
    const resolvedPos = doc.resolve(from)
    const paragraphNode = resolvedPos.parent
    const cursorParagraph = paragraphNode.textContent
      ? truncate(paragraphNode.textContent, 500)
      : undefined

    // Paragraph index
    let cursorParagraphIndex: number | undefined
    let totalParagraphs = 0
    doc.forEach((node, offset, index) => {
      totalParagraphs++
      if (from >= offset && from <= offset + node.nodeSize) {
        cursorParagraphIndex = index
      }
    })

    if (!selectedText && !cursorParagraph) return null

    return {
      selectedText,
      cursorParagraph,
      cursorParagraphIndex,
      totalParagraphs,
    }
  }, [editor])


  const recordHistoryEvent = useCallback(async (
    patch: unknown[],
    actor: 'human' | 'ai',
    proposalId?: string,
    summary?: string
  ) => {
    if (!projectPath || !currentChapterId) return

    const doc = editor?.getJSON()
    const afterHash = doc ? await calculateDocHash(doc) : ''

    const event = {
      schema_version: 1,
      event_id: crypto.randomUUID(),
      created_at: Date.now(),
      actor,
      source_proposal_id: proposalId,
      before_hash: '',
      after_hash: afterHash,
      summary,
      patch,
    }

    await appendAgentHistoryEvent(projectPath, currentChapterId, event)
  }, [projectPath, currentChapterId, editor])

  return {
    getCurrentChapterContent,
    getBlockById: blockOps.getBlockById,
    getSelectedBlocks: blockOps.getSelectedBlocks,
    getCurrentWordCount,
    getEditorState,
    insertBlocksAfter: blockOps.insertBlocksAfter,
    replaceBlock: blockOps.replaceBlock,
    deleteBlocks: blockOps.deleteBlocks,
    highlightBlocks: blockOps.highlightBlocks,
    clearAiHighlights: blockOps.clearAiHighlights,
    recordHistoryEvent,
  }
}

import { useCallback, useEffect } from 'react'
import type { Editor } from '@tiptap/react'

import { eventBus, EVENTS } from '@/lib/events'

import { type MatchResult } from './find-replace-search'
import { navigateToMatch, replaceAllMatches, replaceCurrentMatch } from './find-replace-hooks'

export function useFindReplacePanelEffects(input: {
  editor: Editor | null
  isOpen: boolean
  setShowReplace: (value: boolean) => void
  setFindText: (value: string) => void
  clearDecorations: () => void
  findInputRef: React.RefObject<HTMLInputElement | null>
}) {
  const { editor, isOpen, setShowReplace, setFindText, clearDecorations, findInputRef } = input

  useEffect(() => {
    const handleOpen = () => {
      setShowReplace(true)
    }
    eventBus.on(EVENTS.FIND_REPLACE_OPEN, handleOpen)
    return () => eventBus.off(EVENTS.FIND_REPLACE_OPEN, handleOpen)
  }, [setShowReplace])

  useEffect(() => {
    if (!editor || !isOpen) return

    const { from, to } = editor.state.selection
    if (from !== to) {
      const selectedText = editor.state.doc.textBetween(from, to, '')
      if (selectedText) setFindText(selectedText)
    }

    setTimeout(() => findInputRef.current?.focus(), 50)
  }, [editor, isOpen, setFindText, findInputRef])

  useEffect(() => {
    if (!isOpen) clearDecorations()
  }, [isOpen, clearDecorations])
}

export function useFindReplacePanelHandlers(input: {
  editor: Editor | null
  matches: MatchResult[]
  currentMatch: number
  setCurrentMatch: (value: number) => void
  updateDecorations: (matches: MatchResult[], index: number) => void
  replaceText: string
  clearDecorations: () => void
  onClose: () => void
}) {
  const {
    editor,
    matches,
    currentMatch,
    setCurrentMatch,
    updateDecorations,
    replaceText,
    clearDecorations,
    onClose,
  } = input

  const handleFindNext = useCallback(() => {
    if (!editor || matches.length === 0) return

    const nextIndex = currentMatch % matches.length
    setCurrentMatch(nextIndex + 1)
    updateDecorations(matches, nextIndex)
    navigateToMatch(editor, matches[nextIndex])
  }, [editor, matches, currentMatch, setCurrentMatch, updateDecorations])

  const handleFindPrevious = useCallback(() => {
    if (!editor || matches.length === 0) return

    const prevIndex = (currentMatch - 2 + matches.length) % matches.length
    setCurrentMatch(prevIndex + 1)
    updateDecorations(matches, prevIndex)
    navigateToMatch(editor, matches[prevIndex])
  }, [editor, matches, currentMatch, setCurrentMatch, updateDecorations])

  const handleReplace = useCallback(() => {
    replaceCurrentMatch({ editor, matches, currentMatch, replaceText })
  }, [editor, matches, currentMatch, replaceText])

  const handleReplaceAll = useCallback(() => {
    replaceAllMatches({ editor, matches, replaceText })
  }, [editor, matches, replaceText])

  const handleClose = useCallback(() => {
    clearDecorations()
    onClose()
    editor?.commands.focus()
  }, [clearDecorations, onClose, editor])

  return {
    handleFindNext,
    handleFindPrevious,
    handleReplace,
    handleReplaceAll,
    handleClose,
  }
}

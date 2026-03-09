/**
 * @author Alpha
 * @date 2026-02-11
 * @description 查找替换面板 — 修复位置计算、添加全匹配高亮、正则搜索支持
 */
import { useRef, useState } from 'react'
import type { Editor } from '@tiptap/react'
import { PluginKey } from '@tiptap/pm/state'

import { useFindReplaceMatches, useFindReplacePlugin } from './find-replace-hooks'
import {
  useFindReplacePanelEffects,
  useFindReplacePanelHandlers,
} from './find-replace-panel-hooks'
import { FindReplacePanelView } from './find-replace-render'

interface FindReplacePanelProps {
  editor: Editor | null
  isOpen: boolean
  onClose: () => void
}

const searchHighlightKey = new PluginKey('searchHighlight')

export function FindReplacePanel({ editor, isOpen, onClose }: FindReplacePanelProps) {
  const [findText, setFindText] = useState('')
  const [replaceText, setReplaceText] = useState('')
  const [caseSensitive, setCaseSensitive] = useState(false)
  const [useRegex, setUseRegex] = useState(false)
  const [showReplace, setShowReplace] = useState(true)
  const findInputRef = useRef<HTMLInputElement>(null)

  const pluginRef = useFindReplacePlugin(editor, searchHighlightKey)
  const {
    matches,
    totalMatches,
    currentMatch,
    setCurrentMatch,
    updateDecorations,
    clearDecorations,
  } = useFindReplaceMatches({
    editor,
    isOpen,
    pluginKey: searchHighlightKey,
    pluginRef,
    findText,
    caseSensitive,
    useRegex,
  })

  useFindReplacePanelEffects({
    editor,
    isOpen,
    setShowReplace,
    setFindText,
    clearDecorations,
    findInputRef,
  })

  const {
    handleFindNext,
    handleFindPrevious,
    handleReplace,
    handleReplaceAll,
    handleClose,
  } = useFindReplacePanelHandlers({
    editor,
    matches,
    currentMatch,
    setCurrentMatch,
    updateDecorations,
    replaceText,
    clearDecorations,
    onClose,
  })

  if (!isOpen) return null

  return (
    <FindReplacePanelView
      showReplace={showReplace}
      setShowReplace={setShowReplace}
      findInputRef={findInputRef}
      findText={findText}
      setFindText={setFindText}
      totalMatches={totalMatches}
      currentMatch={currentMatch}
      caseSensitive={caseSensitive}
      setCaseSensitive={setCaseSensitive}
      useRegex={useRegex}
      setUseRegex={setUseRegex}
      onFindPrevious={handleFindPrevious}
      onFindNext={handleFindNext}
      onClose={handleClose}
      replaceText={replaceText}
      setReplaceText={setReplaceText}
      onReplace={handleReplace}
      onReplaceAll={handleReplaceAll}
    />
  )
}

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { Editor } from '@tiptap/react'
import { Fragment } from '@tiptap/pm/model'
import { Plugin, PluginKey } from '@tiptap/pm/state'
import { DecorationSet } from '@tiptap/pm/view'

import { createSearchDecorations, findAllMatches, type MatchResult } from './find-replace-search'

export function useFindReplacePlugin(editor: Editor | null, pluginKey: PluginKey) {
  const pluginRef = useRef<Plugin | null>(null)

  useEffect(() => {
    if (!editor) return

    const plugin = new Plugin({
      key: pluginKey,
      state: {
        init() {
          return DecorationSet.empty
        },
        apply(tr, oldSet) {
          const meta = tr.getMeta(pluginKey)
          if (meta !== undefined) return meta
          if (tr.docChanged) return oldSet.map(tr.mapping, tr.doc)
          return oldSet
        },
      },
      props: {
        decorations(state) {
          return pluginKey.getState(state) || DecorationSet.empty
        },
      },
    })

    pluginRef.current = plugin

    const { state } = editor.view
    const newState = state.reconfigure({ plugins: [...state.plugins, plugin] })
    editor.view.updateState(newState)

    return () => {
      if (editor.view && !editor.isDestroyed) {
        const { state } = editor.view
        const newState = state.reconfigure({
          plugins: state.plugins.filter((p) => p !== plugin),
        })
        editor.view.updateState(newState)
      }
      pluginRef.current = null
    }
  }, [editor, pluginKey])

  return pluginRef
}

function useEditorContentVersion(editor: Editor | null) {
  const [contentVersion, setContentVersion] = useState(0)

  useEffect(() => {
    if (!editor) return

    const handleUpdate = () => {
      setContentVersion((v) => v + 1)
    }

    editor.on('update', handleUpdate)
    return () => {
      editor.off('update', handleUpdate)
    }
  }, [editor])

  return contentVersion
}

function useFindReplaceResults(input: {
  editor: Editor | null
  isOpen: boolean
  findText: string
  caseSensitive: boolean
  useRegex: boolean
}) {
  useEditorContentVersion(input.editor)

  const matches = useMemo(() => {
    if (!input.editor || !input.findText || !input.isOpen) return []
    return findAllMatches(input.editor, input.findText, input.caseSensitive, input.useRegex)
  }, [
    input.caseSensitive,
    input.editor,
    input.findText,
    input.isOpen,
    input.useRegex,
  ])

  return { matches, totalMatches: matches.length }
}

export function useFindReplaceMatches(input: {
  editor: Editor | null
  isOpen: boolean
  pluginKey: PluginKey
  pluginRef: React.RefObject<Plugin | null>
  findText: string
  caseSensitive: boolean
  useRegex: boolean
}) {
  const [currentMatch, setCurrentMatch] = useState(0)

  const updateDecorations = useCallback((matchList: MatchResult[], currentIdx: number) => {
    if (!input.editor || !input.pluginRef.current) return
    const decorations = createSearchDecorations(input.editor, matchList, currentIdx)
    const tr = input.editor.state.tr.setMeta(input.pluginKey, decorations)
    input.editor.view.dispatch(tr)
  }, [input.editor, input.pluginKey, input.pluginRef])

  const clearDecorations = useCallback(() => {
    if (!input.editor || !input.pluginRef.current) return
    const tr = input.editor.state.tr.setMeta(input.pluginKey, DecorationSet.empty)
    input.editor.view.dispatch(tr)
  }, [input.editor, input.pluginKey, input.pluginRef])

  const { matches, totalMatches } = useFindReplaceResults({
    editor: input.editor,
    isOpen: input.isOpen,
    findText: input.findText,
    caseSensitive: input.caseSensitive,
    useRegex: input.useRegex,
  })

  useEffect(() => {
    if (!input.isOpen) {
      clearDecorations()
      return
    }

    if (matches.length === 0) {
      clearDecorations()
      return
    }

    const normalizedIndex = currentMatch > 0 ? (currentMatch - 1) % matches.length : 0
    updateDecorations(matches, normalizedIndex)
  }, [
    clearDecorations,
    currentMatch,
    input.isOpen,
    matches,
    updateDecorations,
  ])

  return {
    matches,
    totalMatches,
    currentMatch,
    setCurrentMatch,
    updateDecorations,
    clearDecorations,
  }
}

export function navigateToMatch(editor: Editor, match: MatchResult) {
  editor.commands.focus()
  editor.commands.setTextSelection({
    from: match.from,
    to: match.to,
  })

  const dom = editor.view.domAtPos(match.from)
  if (dom.node instanceof HTMLElement) {
    dom.node.scrollIntoView({ behavior: 'smooth', block: 'center' })
  } else if (dom.node.parentElement) {
    dom.node.parentElement.scrollIntoView({ behavior: 'smooth', block: 'center' })
  }
}

export function replaceCurrentMatch(input: {
  editor: Editor | null
  matches: MatchResult[]
  currentMatch: number
  replaceText: string
}) {
  if (!input.editor || input.matches.length === 0 || input.currentMatch === 0) return

  const match = input.matches[input.currentMatch - 1]
  const { state, view } = input.editor
  const tr = state.tr.replaceWith(
    match.from,
    match.to,
    input.replaceText ? state.schema.text(input.replaceText) : Fragment.empty,
  )

  view.dispatch(tr)
  input.editor.commands.focus()
}

export function replaceAllMatches(input: {
  editor: Editor | null
  matches: MatchResult[]
  replaceText: string
}) {
  if (!input.editor || input.matches.length === 0) return

  const sortedMatches = [...input.matches].sort((a, b) => b.from - a.from)

  const { state, view } = input.editor
  let tr = state.tr

  sortedMatches.forEach((match) => {
    tr = tr.replaceWith(
      match.from,
      match.to,
      input.replaceText ? state.schema.text(input.replaceText) : Fragment.empty,
    )
  })

  view.dispatch(tr)
  input.editor.commands.focus()
}

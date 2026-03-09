import type { Editor } from '@tiptap/react'
import { Decoration, DecorationSet } from '@tiptap/pm/view'

export interface MatchResult {
  from: number
  to: number
}

export function findAllMatches(
  editor: Editor,
  searchText: string,
  caseSensitive: boolean,
  useRegex: boolean,
): MatchResult[] {
  if (!searchText) return []

  const matches: MatchResult[] = []

  if (useRegex) {
    let regex: RegExp
    try {
      regex = new RegExp(searchText, caseSensitive ? 'g' : 'gi')
    } catch {
      return []
    }

    editor.state.doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return

      let match: RegExpExecArray | null
      regex.lastIndex = 0
      while ((match = regex.exec(node.text)) !== null) {
        if (match[0].length === 0) {
          regex.lastIndex++
          continue
        }
        matches.push({
          from: pos + match.index,
          to: pos + match.index + match[0].length,
        })
      }
    })
  } else {
    const search = caseSensitive ? searchText : searchText.toLowerCase()

    editor.state.doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return

      const text = caseSensitive ? node.text : node.text.toLowerCase()
      let index = 0

      while (index < text.length) {
        const foundIndex = text.indexOf(search, index)
        if (foundIndex === -1) break

        matches.push({
          from: pos + foundIndex,
          to: pos + foundIndex + searchText.length,
        })

        index = foundIndex + 1
      }
    })
  }

  return matches
}

export function createSearchDecorations(
  editor: Editor,
  matches: MatchResult[],
  currentIndex: number,
): DecorationSet {
  if (matches.length === 0) {
    return DecorationSet.empty
  }

  const decorations = matches.map((match, i) => {
    const className = i === currentIndex ? 'search-match-current' : 'search-match'
    return Decoration.inline(match.from, match.to, { class: className })
  })

  return DecorationSet.create(editor.state.doc, decorations)
}

import { createRoot, type Root } from 'react-dom/client'
import type { SuggestionProps } from '@tiptap/suggestion'

import { MentionSuggestion, type MentionSuggestionRef } from './MentionSuggestion'
import type { MentionItem } from './mention-data'

type MentionPopupRendererState = {
  root: Root | null
  container: HTMLDivElement | null
  componentRef: MentionSuggestionRef | null
}

function createRendererState(): MentionPopupRendererState {
  return {
    root: null,
    container: null,
    componentRef: null,
  }
}

export function mountMentionPopup(
  state: MentionPopupRendererState,
  props: SuggestionProps<MentionItem>,
) {
  state.container = document.createElement('div')
  state.container.className = 'mention-suggestion-popup'
  state.container.style.position = 'absolute'
  state.container.style.zIndex = '50'

  state.root = createRoot(state.container)
  document.body.appendChild(state.container)

  positionMentionPopup(state.container, props.clientRect?.())
}

export function updateMentionPopupPosition(
  state: MentionPopupRendererState,
  props: SuggestionProps<MentionItem>,
) {
  if (!state.container) {
    return
  }

  positionMentionPopup(state.container, props.clientRect?.())
}

export function renderMentionPopup(
  state: MentionPopupRendererState,
  props: SuggestionProps<MentionItem>,
  onSelect?: (item: MentionItem) => void,
) {
  if (!state.root) {
    return
  }

  state.root.render(
    <MentionSuggestion
      ref={(ref) => {
        state.componentRef = ref
      }}
      items={props.items as MentionItem[]}
      command={(item) => {
        props.command({
          id: item.id,
          label: item.label,
          mentionType: item.type,
          path: item.path,
        })
        onSelect?.(item)
      }}
    />,
  )
}

export function unmountMentionPopup(state: MentionPopupRendererState) {
  state.root?.unmount()
  state.root = null
  state.container?.remove()
  state.container = null
  state.componentRef = null
}

function positionMentionPopup(container: HTMLDivElement, rect: DOMRect | null | undefined) {
  if (!rect) {
    return
  }

  container.style.left = `${rect.left}px`
  container.style.top = `${rect.bottom + 4}px`

  requestAnimationFrame(() => {
    const currentRect = container.getBoundingClientRect()
    if (currentRect.bottom > window.innerHeight - 10) {
      container.style.top = `${rect.top - currentRect.height - 4}px`
    }
  })
}

export {
  createRendererState,
  type MentionPopupRendererState,
}

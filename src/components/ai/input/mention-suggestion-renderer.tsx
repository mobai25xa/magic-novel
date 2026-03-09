import type { SuggestionKeyDownProps, SuggestionProps } from '@tiptap/suggestion'

import type { MentionItem } from './mention-data'
import {
  createRendererState,
  mountMentionPopup,
  renderMentionPopup,
  unmountMentionPopup,
  updateMentionPopupPosition,
} from './mention-suggestion-renderer-popup'

export function createMentionSuggestionRenderer(onSelect?: (item: MentionItem) => void) {
  return () => {
    const state = createRendererState()

    return {
      onStart(props: SuggestionProps<MentionItem>) {
        mountMentionPopup(state, props)
        renderMentionPopup(state, props, onSelect)
      },

      onUpdate(props: SuggestionProps<MentionItem>) {
        renderMentionPopup(state, props, onSelect)
        updateMentionPopupPosition(state, props)
      },

      onKeyDown(props: SuggestionKeyDownProps) {
        if (props.event.key === 'Escape') {
          unmountMentionPopup(state)
          return true
        }

        return state.componentRef?.onKeyDown(props) ?? false
      },

      onExit() {
        unmountMentionPopup(state)
      },
    }
  }
}

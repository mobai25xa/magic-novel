import { useCallback, useState } from 'react'

import type { ChatContext } from '../../input/chat-context-types'

export function usePanelContexts() {
  const [contexts, setContexts] = useState<ChatContext[]>([])

  const addContext = useCallback((context: ChatContext) => {
    setContexts((prev) => {
      if (prev.some((item) => item.id === context.id)) {
        return prev
      }
      return [...prev, context]
    })
  }, [])

  const removeContext = useCallback((contextId: string) => {
    setContexts((prev) => prev.filter((item) => item.id !== contextId))
  }, [])

  const clearContexts = useCallback(() => {
    setContexts([])
  }, [])

  return {
    contexts,
    addContext,
    removeContext,
    clearContexts,
  }
}

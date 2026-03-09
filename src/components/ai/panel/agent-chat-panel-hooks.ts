import { useEffect } from 'react'

import { observeDroppedFrames } from './agent-chat-panel-metrics'

export function useDroppedFramesMetric(input: {
  sessionId: string
  enabled: boolean
}) {
  const { sessionId, enabled } = input

  useEffect(() => {
    return observeDroppedFrames({ sessionId, enabled })
  }, [enabled, sessionId])
}


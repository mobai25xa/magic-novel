import { useSettingsStore } from '@/state/settings'
import { useStandardAiConsumerState } from '@/features/standard-ai-consumer'

export function usePanelModelState() {
  const standardAi = useStandardAiConsumerState()
  const aiChatViewMode = useSettingsStore((state) => state.aiChatViewMode)
  const setAiChatViewMode = useSettingsStore((state) => state.setAiChatViewMode)

  return {
    ...standardAi,
    aiChatViewMode,
    setAiChatViewMode,
    sessionPersistenceEnabled: true,
  }
}

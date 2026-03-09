import { useAgentChatStore } from '@/state/agent'

export function useAgentChatSessionId() {
  return useAgentChatStore((state) => state.session_id)
}

import { AgentChatPanel } from '@/components/ai/AgentChatPanel'
import { useLayoutStore } from '@/stores/layout-store'

export function RightPanel() {
  const { toggleRightPanel } = useLayoutStore()

  return (
    <div className="panel-sidebar panel-sidebar-right editor-shell-right-panel h-full">
      <AgentChatPanel onClosePanel={toggleRightPanel} />
    </div>
  )
}

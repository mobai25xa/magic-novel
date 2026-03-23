import { AiPanelIconButton } from '@/magic-ui/components'

import { AiStatusBadge } from '@/components/ai/status-badge'

type MissionHeaderProps = {
  liveState: string
  missionId: string
  onRefresh: () => void
  onClose?: () => void
}

export function MissionHeader({ liveState, missionId, onRefresh, onClose }: MissionHeaderProps) {
  return (
    <>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-foreground">Mission</span>
          <AiStatusBadge status={liveState} />
        </div>
        <div className="flex items-center gap-1">
          <AiPanelIconButton onClick={onRefresh} title="Refresh">
            ↻
          </AiPanelIconButton>
          {onClose && (
            <AiPanelIconButton onClick={onClose} title="Close">
              ✕
            </AiPanelIconButton>
          )}
        </div>
      </div>

      <p className="text-xs text-muted-foreground font-mono truncate" title={missionId}>
        {missionId}
      </p>
    </>
  )
}


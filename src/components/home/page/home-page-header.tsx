import { Plus, Settings } from 'lucide-react'

import { WindowControls } from '@/components/layout/WindowControls'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/magic-ui/components'

import { HomeDialogButton } from '../HomeDialogButton'

type Input = {
  createProjectLabel: string
  settingsTitle: string
  onOpenCreateProject: () => void
  onOpenSettings: () => void
}

export function HomePageHeader(input: Input) {
  return (
    <header className="page-header">
      <div className="flex items-center px-4 gap-3 h-full" data-no-drag>
        <img src="/icon.png" alt="Magic Novel" className="w-6 h-6 flex-shrink-0" />
        <h1 className="text-base font-semibold">Magic Novel</h1>
      </div>

      <div className="flex-1 h-full" data-tauri-drag-region />

      <div className="flex gap-2 mr-2" data-no-drag>
        <HomeDialogButton
          onClick={input.onOpenCreateProject}
          size="sm"
        >
          <Plus className="h-4 w-4 mr-1" />
          {input.createProjectLabel}
        </HomeDialogButton>

        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={input.onOpenSettings}
              className="toolbar-btn"
            >
              <Settings className="h-4 w-4" />
            </button>
          </TooltipTrigger>
          <TooltipContent variant="success">{input.settingsTitle}</TooltipContent>
        </Tooltip>
      </div>

      <WindowControls />
    </header>
  )
}

import { Minus, Square, X } from 'lucide-react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

export function WindowControls() {
  const appWindow = getCurrentWindow()
  const { translations } = useTranslation()
  const lt = translations.layout

  const handleMinimize = () => {
    appWindow.minimize()
  }

  const handleMaximize = () => {
    appWindow.toggleMaximize()
  }

  const handleClose = () => {
    appWindow.close()
  }

  return (
    <TooltipProvider>
      <div className="flex items-center h-full" data-no-drag>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={handleMinimize}
              className="window-control"
            >
              <Minus className="h-4 w-4" />
            </button>
          </TooltipTrigger>
          <TooltipContent variant="success">
            {lt.minimize}
          </TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={handleMaximize}
              className="window-control"
            >
              <Square className="h-3.5 w-3.5" />
            </button>
          </TooltipTrigger>
          <TooltipContent variant="success">
            {lt.maximize}
          </TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={handleClose}
              className="window-control window-control-close"
            >
              <X className="h-4 w-4" />
            </button>
          </TooltipTrigger>
          <TooltipContent variant="success">
            {translations.common.close}
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  )
}

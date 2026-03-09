import type { LucideIcon } from 'lucide-react'
import {
  Home,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  Settings,
} from 'lucide-react'

type TopBarAction = {
  key: 'home' | 'left-panel' | 'right-panel' | 'settings'
  title: string
  icon: LucideIcon
  onClick: () => void
}

export function buildTopBarActions(input: {
  isLeftPanelVisible: boolean
  isRightPanelVisible: boolean
  onBackHome: () => void
  onToggleLeftPanel: () => void
  onToggleRightPanel: () => void
  onOpenSettings: () => void
  labels: {
    homePage: string
    hideDirectory: string
    showDirectory: string
    hideAssistant: string
    showAssistant: string
    settings: string
  }
}): TopBarAction[] {
  return [
    {
      key: 'home',
      title: input.labels.homePage,
      icon: Home,
      onClick: input.onBackHome,
    },
    {
      key: 'left-panel',
      title: input.isLeftPanelVisible ? input.labels.hideDirectory : input.labels.showDirectory,
      icon: input.isLeftPanelVisible ? PanelLeftClose : PanelLeftOpen,
      onClick: input.onToggleLeftPanel,
    },
    {
      key: 'right-panel',
      title: input.isRightPanelVisible ? input.labels.hideAssistant : input.labels.showAssistant,
      icon: input.isRightPanelVisible ? PanelRightClose : PanelRightOpen,
      onClick: input.onToggleRightPanel,
    },
    {
      key: 'settings',
      title: input.labels.settings,
      icon: Settings,
      onClick: input.onOpenSettings,
    },
  ]
}

export type { TopBarAction }

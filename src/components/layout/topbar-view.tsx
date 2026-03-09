import { Search } from 'lucide-react'

import { GlobalSearchPanel } from '@/components/common/GlobalSearchPanel'
import { Button, Input, Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/magic-ui/components'

import { WindowControls } from './WindowControls'
import type { TopBarAction } from './topbar-actions'

type TopBarViewProps = {
  projectName?: string
  searchQuery: string
  showSearchResults: boolean
  actions: TopBarAction[]
  searchPlaceholder: string
  onSearchQueryChange: (value: string) => void
  onSearchFocus: () => void
  onSearchBlur: () => void
  onCloseSearch: () => void
  onSearchResultClick: (chapterPath: string) => void
}

export function TopBarView(input: TopBarViewProps) {
  return (
    <TooltipProvider>
      <div className="topbar">
        <div className="flex items-center px-4 gap-3 h-full">
          {input.actions
            .filter((action) => action.key === 'home')
            .map((action) => (
              <Tooltip key={action.key}>
                <TooltipTrigger asChild>
                  <Button onClick={action.onClick} variant="ghost" size="icon">
                    <action.icon className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent variant="success">{action.title}</TooltipContent>
              </Tooltip>
            ))}

          <img src="/icon.png" alt="Magic Novel" className="w-6 h-6 flex-shrink-0" />
          <h1 className="text-base font-semibold">{input.projectName ? `《${input.projectName}》` : 'Magic Novel'}</h1>
        </div>

        <div className="flex-1 flex items-center h-full">
          <div data-tauri-drag-region className="flex-1 h-full min-w-[60px]" />
          <div className="max-w-md w-96 mx-4">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 pointer-events-none" style={{ color: "var(--text-muted-foreground)" }} />
              <Input
                placeholder={input.searchPlaceholder}
                value={input.searchQuery}
                onChange={(event) => input.onSearchQueryChange(event.target.value)}
                onFocus={input.onSearchFocus}
                onBlur={input.onSearchBlur}
                className="h-8 pl-9"
              />
            </div>
          </div>
          <div data-tauri-drag-region className="flex-1 h-full min-w-[60px]" />
        </div>

        <div className="flex items-center gap-2 px-4">
          {input.actions
            .filter((action) => action.key !== 'home')
            .map((action) => (
              <Tooltip key={action.key}>
                <TooltipTrigger asChild>
                  <Button onClick={action.onClick} variant="ghost" size="icon">
                    <action.icon className="h-4 w-4" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent variant="success">{action.title}</TooltipContent>
              </Tooltip>
            ))}
        </div>

        <WindowControls />

        <GlobalSearchPanel
          query={input.searchQuery}
          isOpen={input.showSearchResults && input.searchQuery.trim().length > 0}
          onClose={input.onCloseSearch}
          onResultClick={input.onSearchResultClick}
        />
      </div>
    </TooltipProvider>
  )
}

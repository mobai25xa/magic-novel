import { Check, Code, Copy, Image as ImageIcon, ZoomIn, ZoomOut } from 'lucide-react'

import { cn } from '@/lib/utils'

import type { ChartState, ViewMode } from './mermaid-chart-types'

type MermaidChartToolbarProps = {
  viewMode: ViewMode
  state: ChartState
  copied: boolean
  zoom: number
  onSelectCode: () => void
  onSelectDiagram: () => void
  onZoomOut: () => void
  onZoomIn: () => void
  onCopySvg: () => void
}

export function MermaidChartToolbar(input: MermaidChartToolbarProps) {
  return (
    <div className="flex items-center justify-between px-2 py-1 bg-secondary-50 border-b">
      <div className="flex items-center gap-0.5">
        <ToolbarButton
          active={input.viewMode === 'code'}
          onClick={input.onSelectCode}
          title="Source"
        >
          <Code className="h-3.5 w-3.5" />
        </ToolbarButton>
        <ToolbarButton
          active={input.viewMode === 'diagram'}
          onClick={input.onSelectDiagram}
          title="Diagram"
        >
          <ImageIcon className="h-3.5 w-3.5" />
        </ToolbarButton>
      </div>

      {input.viewMode === 'diagram' && input.state === 'diagram' ? (
        <div className="flex items-center gap-0.5">
          <ToolbarButton onClick={input.onZoomOut} title="Zoom out">
            <ZoomOut className="h-3.5 w-3.5" />
          </ToolbarButton>
          <span className="text-[10px] text-muted-foreground min-w-[3ch] text-center select-none">
            {Math.round(input.zoom * 100)}%
          </span>
          <ToolbarButton onClick={input.onZoomIn} title="Zoom in">
            <ZoomIn className="h-3.5 w-3.5" />
          </ToolbarButton>
          <ToolbarButton onClick={input.onCopySvg} title="Copy SVG">
            {input.copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          </ToolbarButton>
        </div>
      ) : null}
    </div>
  )
}

function ToolbarButton({
  children,
  active,
  onClick,
  title,
}: {
  children: React.ReactNode
  active?: boolean
  onClick: () => void
  title: string
}) {
  return (
    <button
      type="button"
      className={cn(
        'p-1 rounded text-muted-foreground hover:text-secondary-foreground hover-bg transition-colors cursor-pointer',
        active && 'active-bg text-foreground',
      )}
      onClick={onClick}
      title={title}
    >
      {children}
    </button>
  )
}

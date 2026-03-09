import { cn } from '@/lib/utils'
import { CodeBlock } from '@/magic-ui/components'

import type { ChartState } from './mermaid-chart-types'

type MermaidChartContentProps = {
  state: ChartState
  errorMsg: string
  svgHtml: string
  zoom: number
  dragging: boolean
  offset: { x: number; y: number }
  onWheel: (event: React.WheelEvent) => void
  onMouseDown: (event: React.MouseEvent) => void
  onMouseMove: (event: React.MouseEvent) => void
  onMouseUp: () => void
}

export function MermaidChartContent(input: MermaidChartContentProps) {
  return (
    <div
      className={cn(
        'overflow-hidden p-3 min-h-[100px] flex items-center justify-center',
        input.zoom > 1 && (input.dragging ? 'cursor-grabbing' : 'cursor-grab'),
      )}
      onWheel={input.onWheel}
      onMouseDown={input.onMouseDown}
      onMouseMove={input.onMouseMove}
      onMouseUp={input.onMouseUp}
      onMouseLeave={input.onMouseUp}
    >
      {input.state === 'loading' ? (
        <div className="text-xs text-muted-foreground animate-pulse">Rendering...</div>
      ) : null}

      {input.state === 'idle' ? (
        <div className="text-xs text-muted-foreground">Waiting to render...</div>
      ) : null}

      {input.state === 'error' ? (
        <div className="text-xs text-ai-status-error p-2">
          <div className="font-medium mb-1">Mermaid render error</div>
          <CodeBlock className="text-[10px] whitespace-pre-wrap opacity-80">{input.errorMsg}</CodeBlock>
        </div>
      ) : null}

      {input.state === 'diagram' ? (
        <div
          style={{
            transform: `scale(${input.zoom}) translate(${input.offset.x / input.zoom}px, ${input.offset.y / input.zoom}px)`,
            transformOrigin: 'center center',
            transition: input.dragging ? 'none' : 'transform 0.15s ease',
          }}
          dangerouslySetInnerHTML={{ __html: input.svgHtml }}
        />
      ) : null}
    </div>
  )
}

type MermaidChartStreamingProps = {
  code: string
}

export function MermaidChartStreaming(input: MermaidChartStreamingProps) {
  return (
    <CodeBlock className="rounded-md border bg-secondary-50 p-3 text-xs overflow-auto my-2">
      <span className="text-[10px] text-muted-foreground select-none block mb-2">mermaid</span>
      <code>{input.code}</code>
      <span className="inline-block w-1.5 h-3.5 bg-ai-thinking-line animate-pulse ml-0.5 align-middle" />
    </CodeBlock>
  )
}

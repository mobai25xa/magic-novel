import { useEffect, useId, useRef } from 'react'

import { AiChartShell, CodeBlock } from '@/magic-ui/components'
import { MermaidChartContent, MermaidChartStreaming } from './mermaid-chart-content'
import { useMermaidChartInteractions } from './mermaid-chart-interactions'
import { resetMermaidTheme } from './mermaid-loader'
import { useMermaidChartState } from './mermaid-chart-state'
import { MermaidChartToolbar } from './mermaid-chart-toolbar'
import type { MermaidChartProps } from './mermaid-chart-types'
import { useVisible } from '../scroll/use-visible'

export { resetMermaidTheme }

export function MermaidChart(input: MermaidChartProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const visible = useVisible(containerRef, { once: true })
  const uniqueId = useId().replace(/:/g, '_')
  const state = useMermaidChartState({
    ...input,
    uniqueId: `mermaid-${uniqueId}`,
  })

  const interactions = useMermaidChartInteractions({
    zoom: state.zoom,
    dragging: state.dragging,
    setZoom: state.setZoom,
    startDrag: state.startDrag,
    moveDrag: state.moveDrag,
    stopDrag: state.stopDrag,
  })

  const { lastRenderedCode, renderMermaid } = state

  useEffect(() => {
    if (input.streaming || !visible) {
      return
    }

    if (input.code !== lastRenderedCode.current) {
      void renderMermaid(input.code)
    }
  }, [input.code, input.streaming, visible, lastRenderedCode, renderMermaid])

  if (input.streaming) {
    return <MermaidChartStreaming code={input.code} />
  }

  return (
    <AiChartShell ref={containerRef}>
      <MermaidChartToolbar
        viewMode={state.viewMode}
        state={state.state}
        copied={state.copied}
        zoom={state.zoom}
        onSelectCode={() => state.setViewMode('code')}
        onSelectDiagram={() => state.setViewMode('diagram')}
        onZoomOut={interactions.handleZoomOut}
        onZoomIn={interactions.handleZoomIn}
        onCopySvg={state.copySvg}
      />

      {state.viewMode === 'code' ? (
        <CodeBlock className="p-3 text-xs overflow-auto max-h-[400px]">
          <code>{input.code}</code>
        </CodeBlock>
      ) : (
        <MermaidChartContent
          state={state.state}
          errorMsg={state.errorMsg}
          svgHtml={state.svgHtml}
          zoom={state.zoom}
          dragging={state.dragging}
          offset={state.offset}
          onWheel={interactions.handleWheel}
          onMouseDown={interactions.handleMouseDown}
          onMouseMove={interactions.handleMouseMove}
          onMouseUp={interactions.handleMouseUp}
        />
      )}
    </AiChartShell>
  )
}

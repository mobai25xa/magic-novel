import { useCallback, useRef, useState } from 'react'

import type { ChartState, MermaidChartStateInput, ViewMode } from './mermaid-chart-types'
import { getMermaid } from './mermaid-loader'

export function useMermaidChartState(input: MermaidChartStateInput) {
  const [state, setState] = useState<ChartState>('idle')
  const [viewMode, setViewMode] = useState<ViewMode>('diagram')
  const [svgHtml, setSvgHtml] = useState('')
  const [errorMsg, setErrorMsg] = useState('')
  const [copied, setCopied] = useState(false)
  const [zoom, setZoom] = useState(1)
  const [dragging, setDragging] = useState(false)
  const [offset, setOffset] = useState({ x: 0, y: 0 })

  const lastRenderedCode = useRef('')
  const dragStart = useRef({ x: 0, y: 0, ox: 0, oy: 0 })

  const renderMermaid = useCallback(async (source: string) => {
    if (!source.trim()) return
    setState('loading')

    try {
      const mermaid = await getMermaid()
      await mermaid.parse(source)
      const { svg } = await mermaid.render(input.uniqueId, source)
      setSvgHtml(svg)
      setState('diagram')
      lastRenderedCode.current = source
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err))
      setState('error')
    }
  }, [input.uniqueId])


  const copySvg = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(svgHtml)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch {
      // ignore clipboard failures
    }
  }, [svgHtml])

  const startDrag = useCallback((clientX: number, clientY: number) => {
    setDragging(true)
    dragStart.current = { x: clientX, y: clientY, ox: offset.x, oy: offset.y }
  }, [offset.x, offset.y])

  const moveDrag = useCallback((clientX: number, clientY: number) => {
    if (!dragging) return
    setOffset({
      x: dragStart.current.ox + (clientX - dragStart.current.x),
      y: dragStart.current.oy + (clientY - dragStart.current.y),
    })
  }, [dragging])

  const stopDrag = useCallback(() => setDragging(false), [])

  return {
    state,
    viewMode,
    setViewMode,
    svgHtml,
    errorMsg,
    copied,
    zoom,
    setZoom,
    dragging,
    offset,
    copySvg,
    startDrag,
    moveDrag,
    stopDrag,
    renderMermaid,
    lastRenderedCode,
  }
}

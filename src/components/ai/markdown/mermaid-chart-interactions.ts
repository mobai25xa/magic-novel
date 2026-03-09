import { useCallback } from 'react'

import type React from 'react'

import { MAX_ZOOM, MIN_ZOOM, ZOOM_STEP } from './mermaid-chart-types'

type MermaidInteractionsInput = {
  zoom: number
  dragging: boolean
  setZoom: React.Dispatch<React.SetStateAction<number>>
  startDrag: (clientX: number, clientY: number) => void
  moveDrag: (clientX: number, clientY: number) => void
  stopDrag: () => void
}

export function useMermaidChartInteractions(input: MermaidInteractionsInput) {
  const handleZoomIn = useCallback(() => {
    input.setZoom((value) => Math.min(MAX_ZOOM, value + ZOOM_STEP))
  }, [input])

  const handleZoomOut = useCallback(() => {
    input.setZoom((value) => Math.max(MIN_ZOOM, value - ZOOM_STEP))
  }, [input])

  const handleWheel = useCallback((event: React.WheelEvent) => {
    if (!event.ctrlKey && !event.metaKey) {
      return
    }

    event.preventDefault()
    input.setZoom((value) => {
      const delta = event.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP
      return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, value + delta))
    })
  }, [input])

  const handleMouseDown = useCallback((event: React.MouseEvent) => {
    if (input.zoom <= 1) {
      return
    }
    input.startDrag(event.clientX, event.clientY)
  }, [input])

  const handleMouseMove = useCallback((event: React.MouseEvent) => {
    if (!input.dragging) {
      return
    }
    input.moveDrag(event.clientX, event.clientY)
  }, [input])

  const handleMouseUp = useCallback(() => {
    input.stopDrag()
  }, [input])

  return {
    handleZoomIn,
    handleZoomOut,
    handleWheel,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
  }
}

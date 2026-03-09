export type MermaidChartProps = {
  code: string
  streaming?: boolean
}

export type MermaidChartStateInput = MermaidChartProps & {
  uniqueId: string
}

export type ChartState = 'idle' | 'loading' | 'diagram' | 'error'
export type ViewMode = 'code' | 'diagram'

export const MIN_ZOOM = 0.5
export const MAX_ZOOM = 3
export const ZOOM_STEP = 0.2

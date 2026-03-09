import { logUiMetric } from '@/agent/telemetry'

export function observeDroppedFrames(input: {
  sessionId: string
  enabled: boolean
  sampleMs?: number
}) {
  const sampleMs = input.sampleMs ?? 1000
  if (!input.enabled || typeof window === 'undefined') {
    return () => {}
  }

  let frameCount = 0
  let rafId = 0
  let timerId = 0

  const onFrame = () => {
    frameCount += 1
    rafId = window.requestAnimationFrame(onFrame)
  }

  rafId = window.requestAnimationFrame(onFrame)
  timerId = window.setInterval(() => {
    const expected = Math.round((sampleMs / 1000) * 60)
    const dropped = Math.max(0, expected - frameCount)
    logUiMetric({
      sessionId: input.sessionId,
      metric: 'dropped_frames_estimate',
      value: dropped,
      tags: { sampleMs },
    })
    frameCount = 0
  }, sampleMs)

  return () => {
    window.cancelAnimationFrame(rafId)
    window.clearInterval(timerId)
  }
}

export function reportUiCounter(input: {
  sessionId: string
  metric: 'inline_diff_open_rate' | 'step_retry_click_rate'
  turnId?: number
  tags?: Record<string, string | number | boolean>
}) {
  logUiMetric({
    sessionId: input.sessionId,
    turnId: input.turnId,
    metric: input.metric,
    value: 1,
    tags: input.tags,
  })
}

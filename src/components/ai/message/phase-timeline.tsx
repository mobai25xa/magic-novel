import { useCallback, useEffect, useRef, useState } from 'react'

import type { AgentUiTurnPhase } from '@/lib/agent-chat/types'
import type { LoadingStage } from '@/lib/agent-chat/timeline'
import { cn } from '@/lib/utils'

import { useAiTranslations } from '../ai-hooks'
import { Spinner } from '@/magic-ui/components'

type PhaseEntry = {
  key: string
  label: string
}

type PhaseTimelineProps = {
  phase: AgentUiTurnPhase
  stage: LoadingStage
  /** Whether the agent loop is actively processing (controls entry addition). */
  running: boolean
}

const MIN_DISPLAY_MS = 800
const INACTIVE_CLEAR_DELAY_MS = 1200
const FADE_OUT_MS = 500
const MAX_PHASE_ENTRIES = 3

function isTerminalPhase(phase: AgentUiTurnPhase) {
  return phase === 'completed' || phase === 'cancelled' || phase === 'failed'
}

function resolveLabel(
  ai: ReturnType<typeof useAiTranslations>,
  phase: AgentUiTurnPhase,
  stage: LoadingStage,
): string {
  if (stage === 'thinking') return ai.turn.stageCopy.thinking
  if (stage === 'streaming') return ai.turn.stageCopy.streaming
  return ai.turn.phaseCopy[phase] || ai.turn.stageCopy.response
}

function usePhaseHistory(
  ai: ReturnType<typeof useAiTranslations>,
  phase: AgentUiTurnPhase,
  stage: LoadingStage,
  running: boolean,
) {
  const [entries, setEntries] = useState<PhaseEntry[]>([])
  const [exiting, setExiting] = useState(false)

  const seqRef = useRef(0)
  const prevLabelRef = useRef('')
  const lastFlushRef = useRef(0)

  const queueRef = useRef<PhaseEntry[]>([])
  const drainTimerRef = useRef<number | null>(null)
  const exitTimerRef = useRef<number | null>(null)

  const clearDrain = useCallback(() => {
    if (drainTimerRef.current !== null) {
      window.clearTimeout(drainTimerRef.current)
      drainTimerRef.current = null
    }
  }, [])

  const resetTimeline = useCallback(() => {
    clearDrain()
    queueRef.current = []
    setEntries([])
    setExiting(false)
    prevLabelRef.current = ''
    lastFlushRef.current = 0
  }, [clearDrain])

  const clearExit = useCallback(() => {
    if (exitTimerRef.current !== null) {
      window.clearTimeout(exitTimerRef.current)
      exitTimerRef.current = null
    }
  }, [])

  const scheduleExit = useCallback((delay: number) => {
    clearExit()

    const startExit = () => {
      if (queueRef.current.length > 0 || drainTimerRef.current !== null) {
        exitTimerRef.current = window.setTimeout(startExit, 150)
        return
      }

      setExiting(true)
      exitTimerRef.current = window.setTimeout(() => {
        resetTimeline()
      }, FADE_OUT_MS)
    }

    exitTimerRef.current = window.setTimeout(startExit, delay)
  }, [clearExit, resetTimeline])

  // --- Queue drain: flush one entry at a time with MIN_DISPLAY_MS gap ---
  const drainQueue = useCallback(() => {
    if (drainTimerRef.current !== null) return

    const processNext = () => {
      const next = queueRef.current.shift()
      if (!next) {
        drainTimerRef.current = null
        return
      }
      setExiting(false)
      setEntries((prev) => [...prev, next].slice(-MAX_PHASE_ENTRIES))
      lastFlushRef.current = Date.now()

      if (queueRef.current.length > 0) {
        drainTimerRef.current = window.setTimeout(processNext, MIN_DISPLAY_MS)
      } else {
        drainTimerRef.current = null
      }
    }

    const elapsed = Date.now() - lastFlushRef.current
    const delay = Math.max(0, MIN_DISPLAY_MS - elapsed)
    if (delay === 0) {
      processNext()
    } else {
      drainTimerRef.current = window.setTimeout(processNext, delay)
    }
  }, [])

  // --- Enqueue when phase/stage changes while running ---
  useEffect(() => {
    if (!running) return

    clearExit()

    const label = resolveLabel(ai, phase, stage)
    if (label === prevLabelRef.current) return
    prevLabelRef.current = label

    const seq = seqRef.current
    seqRef.current += 1
    queueRef.current.push({ key: `${phase}_${stage}_${seq}`, label })
    drainQueue()
  }, [ai, phase, stage, running, clearExit, drainQueue])

  // --- Exit on terminal phase or when processing stops ---
  useEffect(() => {
    if (isTerminalPhase(phase)) {
      scheduleExit(0)
      return
    }

    if (!running && (entries.length > 0 || queueRef.current.length > 0)) {
      scheduleExit(INACTIVE_CLEAR_DELAY_MS)
      return
    }

    clearExit()
  }, [phase, running, entries.length, clearExit, scheduleExit])

  // --- Cleanup ---
  useEffect(() => () => {
    clearDrain()
    clearExit()
  }, [clearDrain, clearExit])

  return { entries, exiting }
}

export function PhaseTimeline({ phase, stage, running }: PhaseTimelineProps) {
  const ai = useAiTranslations()
  const { entries, exiting } = usePhaseHistory(ai, phase, stage, running)

  if (entries.length === 0) return null

  const lastIndex = entries.length - 1

  return (
    <div
      className={cn(
        'ai-phase-timeline',
        exiting && 'ai-animate-phase-timeline-out',
      )}
      aria-live="polite"
    >
      {entries.map((entry, index) => {
        const isLatest = index === lastIndex
        const levelClass = isLatest
          ? 'ai-phase-item-current'
          : index === lastIndex - 1
            ? 'ai-phase-item-prev'
            : 'ai-phase-item-old'

        return (
          <div
            key={entry.key}
            className={cn(
              'ai-phase-item',
              levelClass,
              isLatest && 'ai-animate-phase-line-in',
            )}
          >
            {isLatest && !exiting ? (
              <span className="ai-phase-icon ai-phase-icon-running" aria-hidden="true">
                <Spinner size="xs" className="text-ai-status-running" />
              </span>
            ) : (
              <span className="ai-phase-icon ai-phase-icon-done" aria-hidden="true">✓</span>
            )}
            <span className="ai-phase-label">{entry.label}</span>
          </div>
        )
      })}
    </div>
  )
}

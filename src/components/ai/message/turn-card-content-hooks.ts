import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import type { AgentUiTurnState } from '@/lib/agent-chat/types'

const TYPEWRITER_LINE_THRESHOLD = 40
const TYPEWRITER_CHUNK_SIZE = 20
const TYPEWRITER_MIN_DELAY_MS = 30
const TYPEWRITER_MAX_DELAY_MS = 80

function splitTypewriterChunks(text: string): string[] {
  if (!text) {
    return []
  }

  const chunks: string[] = []
  const lines = text.split('\n')

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex] ?? ''

    if (line.length > TYPEWRITER_LINE_THRESHOLD) {
      for (let cursor = 0; cursor < line.length; cursor += TYPEWRITER_CHUNK_SIZE) {
        chunks.push(line.slice(cursor, cursor + TYPEWRITER_CHUNK_SIZE))
      }
    } else if (line.length > 0) {
      chunks.push(line)
    }

    if (lineIndex < lines.length - 1) {
      chunks.push('\n')
    }
  }

  return chunks
}

function randomTypewriterDelay() {
  const jitter = TYPEWRITER_MAX_DELAY_MS - TYPEWRITER_MIN_DELAY_MS
  return TYPEWRITER_MIN_DELAY_MS + Math.floor(Math.random() * (jitter + 1))
}

function resolveThinkingText(tick: number) {
  const dots = '.'.repeat((tick % 3) + 1)
  return `Thinking${dots}`
}

function computeElapsedMs(input: {
  running: boolean
  clockNow: number
  turn: AgentUiTurnState
}) {
  const runningEnd = input.running ? input.clockNow : input.turn.updatedAt
  const end = input.turn.finishedAt ?? runningEnd
  return Math.max(0, end - input.turn.startedAt)
}

function useRunningClock(running: boolean) {
  const [clockNow, setClockNow] = useState(() => Date.now())

  useEffect(() => {
    if (!running) {
      return
    }

    const timer = window.setInterval(() => {
      setClockNow(Date.now())
    }, 100)

    return () => {
      window.clearInterval(timer)
    }
  }, [running])

  return clockNow
}

function useThinkingTick(running: boolean) {
  const [thinkingTick, setThinkingTick] = useState(0)

  useEffect(() => {
    if (!running) {
      return
    }

    const timer = window.setInterval(() => {
      setThinkingTick((value) => value + 1)
    }, 360)

    return () => {
      window.clearInterval(timer)
    }
  }, [running])

  return thinkingTick
}

function useTypewriterAnswer(answerText: string, running: boolean) {
  const [typedAnswer, setTypedAnswer] = useState('')
  const sourceTextRef = useRef('')
  const queueRef = useRef<string[]>([])
  const timerRef = useRef<number | null>(null)
  const runningRef = useRef(running)

  useEffect(() => {
    runningRef.current = running
  }, [running])

  const stopPump = useCallback(() => {
    if (timerRef.current !== null) {
      window.clearTimeout(timerRef.current)
      timerRef.current = null
    }
  }, [])

  const pumpQueue = useCallback(() => {
    if (timerRef.current !== null) {
      return
    }

    const flushNext = () => {
      const nextChunk = queueRef.current.shift()
      if (typeof nextChunk !== 'string') {
        timerRef.current = null
        return
      }

      setTypedAnswer((prev) => `${prev}${nextChunk}`)

      if (queueRef.current.length === 0) {
        timerRef.current = null
        return
      }

      timerRef.current = window.setTimeout(flushNext, randomTypewriterDelay())
    }

    flushNext()
  }, [])

  useEffect(() => {
    if (!runningRef.current) {
      return
    }

    if (!answerText) {
      stopPump()
      queueRef.current = []
      sourceTextRef.current = ''
      const clearTimer = window.setTimeout(() => {
        setTypedAnswer('')
      }, 0)
      return () => window.clearTimeout(clearTimer)
    }

    const previousSource = sourceTextRef.current
    if (!answerText.startsWith(previousSource)) {
      stopPump()
      queueRef.current = []
      sourceTextRef.current = answerText
      const resetTimer = window.setTimeout(() => {
        setTypedAnswer(answerText)
      }, 0)
      return () => window.clearTimeout(resetTimer)
    }

    const delta = answerText.slice(previousSource.length)
    sourceTextRef.current = answerText

    if (!delta) {
      return
    }

    queueRef.current.push(...splitTypewriterChunks(delta))
    pumpQueue()
  }, [answerText, pumpQueue, stopPump])

  useEffect(() => {
    if (running) {
      return
    }

    stopPump()
    queueRef.current = []
    sourceTextRef.current = answerText
    const syncTimer = window.setTimeout(() => {
      setTypedAnswer(answerText)
    }, 0)

    return () => window.clearTimeout(syncTimer)
  }, [running, answerText, stopPump])

  useEffect(() => () => {
    stopPump()
  }, [stopPump])

  return typedAnswer
}

export function useTurnCardContentModel(input: {
  text: string
  turn: AgentUiTurnState
  running: boolean
}) {
  const clockNow = useRunningClock(input.running)
  const thinkingTick = useThinkingTick(input.running)
  const typedAnswer = useTypewriterAnswer(input.text, input.running)

  return useMemo(() => {
    const elapsedMs = computeElapsedMs({
      running: input.running,
      clockNow,
      turn: input.turn,
    })

    const hasAnswer = Boolean(typedAnswer.trim())
    const rawHasAnswer = Boolean(input.text.trim())
    const thinkingText = input.running ? resolveThinkingText(thinkingTick) : ''

    return {
      typedAnswer,
      elapsedMs,
      hasAnswer,
      rawHasAnswer,
      thinkingText,
      isStreaming: input.running || (rawHasAnswer && typedAnswer !== input.text),
    }
  }, [clockNow, input.running, input.text, input.turn, thinkingTick, typedAnswer])
}

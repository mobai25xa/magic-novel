import type {
  AgentUiEventType,
  AgentUiTimelineEvent,
  AgentUiToolStep,
  AgentUiTurnPhase,
} from '../types'
import type { LoadingStage, TimelineBlock, TurnTimelineSnapshot } from './timeline-types'

export type BuildTimelineInput = {
  turn: number
  events: AgentUiTimelineEvent[]
  toolStepsByCallId: Record<string, AgentUiToolStep>
  answerText?: string
  thinkingText?: string
  running: boolean
  phase: AgentUiTurnPhase
}

function normalizeNumber(value: unknown) {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function fallbackSeq(event: AgentUiTimelineEvent, index: number) {
  const raw = normalizeNumber(event.seq)
  if (raw === undefined) {
    return index + 1
  }
  return Math.max(1, Math.floor(raw))
}

function fallbackTs(event: AgentUiTimelineEvent, index: number) {
  return normalizeNumber(event.ts) ?? index + 1
}

function sortEventsBySeq(events: AgentUiTimelineEvent[]) {
  return events
    .map((event, index) => ({
      event,
      seq: fallbackSeq(event, index),
      ts: fallbackTs(event, index),
      index,
    }))
    .sort((a, b) => {
      if (a.seq !== b.seq) {
        return a.seq - b.seq
      }
      if (a.ts !== b.ts) {
        return a.ts - b.ts
      }
      return a.index - b.index
    })
}

function stripThinkingPrefix(answer: string, thinking: string) {
  const normalizedAnswer = answer.trimStart()
  const normalizedThinking = thinking.trim()
  if (!normalizedThinking) {
    return answer
  }

  if (normalizedAnswer.startsWith(normalizedThinking)) {
    const prefixIndex = answer.indexOf(normalizedThinking)
    if (prefixIndex >= 0) {
      return answer.slice(prefixIndex + normalizedThinking.length).trimStart()
    }
  }

  return answer
}

function normalizeStage(input: {
  sortedEvents: Array<{ event: AgentUiTimelineEvent }>
  running: boolean
}): LoadingStage {
  if (!input.running) {
    return 'response'
  }

  const last = input.sortedEvents[input.sortedEvents.length - 1]?.event
  if (!last) {
    return 'response'
  }

  if (last.type === 'ASSISTANT_TEXT_DELTA') {
    return 'streaming'
  }

  if (last.type === 'THINKING_TEXT_DELTA') {
    return 'thinking'
  }

  return 'response'
}

type MutableAssistantSegment = {
  seqStart: number
  seqEnd: number
  ts: number
  text: string
}

type MutableThinkingSegment = {
  seqStart: number
  seqEnd: number
  ts: number
  hasContent: boolean
}

function buildBlocks(input: {
  events: AgentUiTimelineEvent[]
  answerText: string
  thinkingText: string
}): TimelineBlock[] {
  const sorted = sortEventsBySeq(input.events)
  const blocks: TimelineBlock[] = []

  const toolCallSeen = new Set<string>()
  let assistantIndex = 0
  let assistantSegment: MutableAssistantSegment | null = null
  let thinkingSegment: MutableThinkingSegment | null = null

  const flushAssistantSegment = () => {
    if (!assistantSegment || !assistantSegment.text.trim()) {
      assistantSegment = null
      return
    }

    blocks.push({
      id: `assistant_segment_${assistantIndex}`,
      type: 'assistant_segment',
      seqStart: assistantSegment.seqStart,
      seqEnd: assistantSegment.seqEnd,
      ts: assistantSegment.ts,
      text: assistantSegment.text,
    })

    assistantIndex += 1
    assistantSegment = null
  }

  const toolTriggerTypes = new Set<AgentUiEventType>([
    'TOOL_CALL_STARTED',
    'WAITING_FOR_CONFIRMATION',
    'ASKUSER_REQUESTED',
  ])

  for (const item of sorted) {
    const { event, seq, ts } = item

    if (event.type === 'ASSISTANT_TEXT_DELTA') {
      const delta = event.delta ?? event.summary ?? ''
      if (!delta) {
        continue
      }

      if (!assistantSegment) {
        assistantSegment = {
          seqStart: seq,
          seqEnd: seq,
          ts,
          text: delta,
        }
      } else {
        assistantSegment.seqEnd = seq
        assistantSegment.text += delta
      }
      continue
    }

    if (event.type === 'THINKING_TEXT_DELTA') {
      if (!thinkingSegment) {
        thinkingSegment = {
          seqStart: seq,
          seqEnd: seq,
          ts,
          hasContent: Boolean(event.delta?.trim() || event.summary?.trim()),
        }
      } else {
        thinkingSegment.seqEnd = seq
        thinkingSegment.hasContent = thinkingSegment.hasContent || Boolean(event.delta?.trim() || event.summary?.trim())
      }
      continue
    }

    if (toolTriggerTypes.has(event.type)) {
      flushAssistantSegment()
      const callId = event.callId
      if (!callId || toolCallSeen.has(callId)) {
        continue
      }
      toolCallSeen.add(callId)
      blocks.push({
        id: `tool_call_${callId}`,
        type: 'tool_call',
        seq,
        ts,
        callId,
      })
      continue
    }

    flushAssistantSegment()
  }

  flushAssistantSegment()

  if (thinkingSegment) {
    blocks.push({
      id: 'thinking_panel',
      type: 'thinking_panel',
      seqStart: thinkingSegment.seqStart,
      seqEnd: thinkingSegment.seqEnd,
      ts: thinkingSegment.ts,
      hasContent: thinkingSegment.hasContent,
    })
  }

  const sanitizedAnswer = stripThinkingPrefix(input.answerText, input.thinkingText)
  if (sanitizedAnswer.trim()) {
    const assistants = blocks
      .filter((block): block is Extract<TimelineBlock, { type: 'assistant_segment' }> => block.type === 'assistant_segment')
      .sort((a, b) => (a.seqStart === b.seqStart ? a.seqEnd - b.seqEnd : a.seqStart - b.seqStart))

    if (assistants.length === 0) {
      const fallback = sorted[sorted.length - 1]
      blocks.push({
        id: 'assistant_segment_fallback',
        type: 'assistant_segment',
        seqStart: fallback?.seq ?? 1,
        seqEnd: fallback?.seq ?? 1,
        ts: fallback?.ts ?? 1,
        text: sanitizedAnswer,
      })
    } else {
      const merged = assistants.map((item) => item.text).join('')
      if (sanitizedAnswer.startsWith(merged) && sanitizedAnswer.length > merged.length) {
        const last = assistants[assistants.length - 1]
        const target = blocks.find((item) => item.id === last.id)
        const tail = sanitizedAnswer.slice(merged.length)
        if (target && target.type === 'assistant_segment' && tail) {
          target.text = `${target.text}${tail}`
        }
      }
    }
  }

  return blocks
    .slice()
    .sort((a, b) => {
      const seqA = a.type === 'tool_call' ? a.seq : a.seqStart
      const seqB = b.type === 'tool_call' ? b.seq : b.seqStart
      if (seqA !== seqB) {
        return seqA - seqB
      }
      if (a.ts !== b.ts) {
        return a.ts - b.ts
      }
      return a.id.localeCompare(b.id)
    })
}

export function buildTurnTimelineSnapshot(input: BuildTimelineInput): TurnTimelineSnapshot {
  const sortedEvents = sortEventsBySeq(input.events)
  const blocks = buildBlocks({
    events: input.events,
    answerText: input.answerText ?? '',
    thinkingText: input.thinkingText ?? '',
  })

  return {
    version: 2,
    turn: input.turn,
    blocks,
    stage: normalizeStage({
      sortedEvents,
      running: input.running,
    }),
    createdAt: Date.now(),
  }
}

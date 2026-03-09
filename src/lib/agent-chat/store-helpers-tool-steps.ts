import type { AgentAskUserAnswer } from '@/agent/types'

import type {
  AgentUiToolStep,
  ChatToolTrace,
} from './types'
import type { AgentChatStateSlice } from './store-helpers'
import { ensureTurnState } from './store-helpers'
import {
  buildToolStepOutput,
  redactValue,
  summarizeArgs,
} from './tool-step-utils'

export interface ToolStepStartInput {
  callId: string
  llmCallId?: string
  toolName: string
  args: Record<string, unknown>
  ts?: number
}

export interface ToolStepProgressInput {
  callId: string
  llmCallId?: string
  toolName: string
  progress: string
  ts?: number
}

export interface ToolStepCompleteInput {
  callId: string
  llmCallId?: string
  toolName: string
  output: string
  trace: ChatToolTrace
  ts?: number
}

export interface ToolStepWaitingInput {
  callId: string
  llmCallId?: string
  toolName: string
  waitState?: 'waiting_confirmation' | 'waiting_askuser'
  ts?: number
}

export interface ToolStepAnsweredInput {
  callId: string
  toolName?: string
  answers: AgentAskUserAnswer[]
  ts?: number
}

function mapTraceStatusToStepStatus(trace: ChatToolTrace): AgentUiToolStep['status'] {
  return trace.status === 'ok' ? 'success' : 'error'
}

function sortSteps(steps: AgentUiToolStep[]) {
  return [...steps].sort((a, b) => {
    if (a.startedAt !== b.startedAt) {
      return a.startedAt - b.startedAt
    }
    return a.callId.localeCompare(b.callId)
  })
}

function mapProgressToStepStatus(progress: string): AgentUiToolStep['status'] {
  return progress === 'waiting_confirmation' || progress === 'waiting_askuser'
    ? 'waiting_confirmation'
    : 'running'
}

export function reduceUpsertToolStepStarted(
  state: AgentChatStateSlice,
  turn: number,
  input: ToolStepStartInput,
): Partial<AgentChatStateSlice> {
  const ts = input.ts ?? Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'tool_running', ts)
  const current = state.stepsByTurnId[turn] || []
  const index = current.findIndex((step) => step.callId === input.callId)

  const argsSummary = summarizeArgs(input.toolName, input.args)
  const inputPreview = redactValue(input.args)

  const nextStep: AgentUiToolStep = index >= 0
    ? {
      ...current[index],
      llmCallId: input.llmCallId ?? current[index].llmCallId,
      toolName: input.toolName,
      status: current[index].status === 'waiting_confirmation' ? 'waiting_confirmation' : 'running',
      startedAt: Math.min(current[index].startedAt, ts),
      finishedAt: undefined,
      durationMs: undefined,
      progress: 'started',
      argsSummary: current[index].argsSummary || argsSummary,
      inputPreview: current[index].inputPreview ?? inputPreview,
      summary: `${input.toolName} · running`,
    }
    : {
      callId: input.callId,
      llmCallId: input.llmCallId,
      toolName: input.toolName,
      status: 'running',
      startedAt: ts,
      progress: 'started',
      argsSummary,
      inputPreview,
      summary: `${input.toolName} · running`,
    }

  const next = index >= 0
    ? current.map((step, idx) => (idx === index ? nextStep : step))
    : [...current, nextStep]

  return {
    stepsByTurnId: {
      ...state.stepsByTurnId,
      [turn]: sortSteps(next),
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase: 'tool_running',
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

export function reduceUpsertToolStepProgress(
  state: AgentChatStateSlice,
  turn: number,
  input: ToolStepProgressInput,
): Partial<AgentChatStateSlice> {
  const ts = input.ts ?? Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'tool_running', ts)
  const current = state.stepsByTurnId[turn] || []
  const index = current.findIndex((step) => step.callId === input.callId)

  const nextStep: AgentUiToolStep = index >= 0
    ? {
      ...current[index],
      llmCallId: input.llmCallId ?? current[index].llmCallId,
      toolName: input.toolName,
      status: mapProgressToStepStatus(input.progress),
      progress: input.progress,
      summary: `${input.toolName} · ${input.progress}`,
    }
    : {
      callId: input.callId,
      llmCallId: input.llmCallId,
      toolName: input.toolName,
      status: 'running',
      startedAt: ts,
      progress: input.progress,
      summary: `${input.toolName} · ${input.progress}`,
    }

  const next = index >= 0
    ? current.map((step, idx) => (idx === index ? nextStep : step))
    : [...current, nextStep]

  return {
    stepsByTurnId: {
      ...state.stepsByTurnId,
      [turn]: sortSteps(next),
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase: 'tool_running',
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

export function reduceUpsertToolStepCompleted(
  state: AgentChatStateSlice,
  turn: number,
  input: ToolStepCompleteInput,
): Partial<AgentChatStateSlice> {
  const ts = input.ts ?? Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'tool_running', ts)
  const current = state.stepsByTurnId[turn] || []
  const index = current.findIndex((step) => step.callId === input.callId)
  const existing = index >= 0 ? current[index] : undefined

  const status = mapTraceStatusToStepStatus(input.trace)
  const startedAt = existing?.startedAt ?? ts
  const outputMeta = buildToolStepOutput({
    toolName: input.toolName,
    output: input.output,
    trace: input.trace,
  })

  const nextStep: AgentUiToolStep = {
    callId: input.callId,
    llmCallId: input.llmCallId ?? existing?.llmCallId,
    toolName: input.toolName,
    status,
    startedAt,
    finishedAt: ts,
    durationMs: input.trace.duration_ms || Math.max(0, ts - startedAt),
    argsSummary: existing?.argsSummary,
    resultSummary: outputMeta.resultSummary,
    progress: existing?.progress,
    inputPreview: existing?.inputPreview,
    outputPreview: outputMeta.outputPreview,
    rawOutput: outputMeta.rawOutput,
    retryable: outputMeta.retryable,
    errorMessage: outputMeta.errorMessage,
    errorCode: input.trace.error_code,
    faultDomain: input.trace.fault_domain,
    stage: input.trace.stage,
    revisionBefore: input.trace.revision_before,
    revisionAfter: input.trace.revision_after,
    txId: input.trace.tx_id,
    summary: outputMeta.resultSummary,
  }

  const next = index >= 0
    ? current.map((step, idx) => (idx === index ? nextStep : step))
    : [...current, nextStep]

  return {
    stepsByTurnId: {
      ...state.stepsByTurnId,
      [turn]: sortSteps(next),
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase: 'tool_running',
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

export function reduceMarkAskUserStepAnswered(
  state: AgentChatStateSlice,
  turn: number,
  input: ToolStepAnsweredInput,
): Partial<AgentChatStateSlice> {
  const ts = input.ts ?? Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'tool_running', ts)
  const current = state.stepsByTurnId[turn] || []
  const index = current.findIndex((step) => step.callId === input.callId)
  const existing = index >= 0 ? current[index] : undefined
  const toolName = existing?.toolName ?? input.toolName ?? 'askuser'

  const nextStep: AgentUiToolStep = {
    callId: input.callId,
    llmCallId: existing?.llmCallId,
    toolName,
    status: 'running',
    startedAt: existing?.startedAt ?? ts,
    finishedAt: undefined,
    durationMs: undefined,
    argsSummary: existing?.argsSummary,
    resultSummary: `${toolName} · answered`,
    progress: 'answered',
    inputPreview: existing?.inputPreview,
    outputPreview: redactValue({ answers: input.answers }),
    rawOutput: JSON.stringify({ answers: input.answers }),
    retryable: existing?.retryable,
    errorMessage: undefined,
    errorCode: undefined,
    faultDomain: existing?.faultDomain,
    stage: existing?.stage,
    revisionBefore: existing?.revisionBefore,
    revisionAfter: existing?.revisionAfter,
    txId: existing?.txId,
    summary: `${toolName} · answered`,
  }

  const next = index >= 0
    ? current.map((step, idx) => (idx === index ? nextStep : step))
    : [...current, nextStep]

  return {
    stepsByTurnId: {
      ...state.stepsByTurnId,
      [turn]: sortSteps(next),
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase: 'tool_running',
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

export function reduceMarkWaitingForConfirmation(
  state: AgentChatStateSlice,
  turn: number,
  input: ToolStepWaitingInput,
): Partial<AgentChatStateSlice> {
  const ts = input.ts ?? Date.now()
  const ensured = ensureTurnState(state.turnById, state.turnOrder, turn, 'tool_running', ts)
  const steps = state.stepsByTurnId[turn] || []
  const index = steps.findIndex((step) => step.callId === input.callId)

  const waitState = input.waitState ?? 'waiting_confirmation'
  const nextStep: AgentUiToolStep = index >= 0
    ? {
      ...steps[index],
      llmCallId: input.llmCallId ?? steps[index].llmCallId,
      toolName: input.toolName,
      status: 'waiting_confirmation',
      finishedAt: undefined,
      progress: waitState,
      summary: `${input.toolName} · ${waitState}`,
    }
    : {
      callId: input.callId,
      llmCallId: input.llmCallId,
      toolName: input.toolName,
      status: 'waiting_confirmation',
      startedAt: ts,
      progress: waitState,
      summary: `${input.toolName} · ${waitState}`,
    }

  const next = index >= 0
    ? steps.map((step, idx) => (idx === index ? nextStep : step))
    : [...steps, nextStep]

  return {
    stepsByTurnId: {
      ...state.stepsByTurnId,
      [turn]: sortSteps(next),
    },
    turnById: {
      ...ensured.turnById,
      [turn]: {
        ...ensured.turnById[turn],
        phase: 'tool_running',
        updatedAt: ts,
      },
    },
    turnOrder: ensured.turnOrder,
  }
}

import {
  aiOpenAiChatCompletionClient,
  getOpenAiProviderSettingsClient,
  saveOpenAiProviderSettingsClient,
} from '@/platform/tauri/clients'
import { agentTurnResumeClient } from '@/platform/tauri/clients/agent-engine-client'
import { formatUnknownError } from '@/lib/error-utils'
import {
  appendChapterHistoryEvent,
  getOpenAiProviderSettings,
  saveOpenAiProviderSettings,
  missionCreate as missionCreateCommand,
  missionStart as missionStartCommand,
  missionPause as missionPauseCommand,
  missionResume as missionResumeCommand,
  missionCancel as missionCancelCommand,
  missionGetStatus as missionGetStatusCommand,
  missionList as missionListCommand,
  type MissionCreateInput,
  type MissionCreateOutput,
  type MissionStartInput,
  type OpenAiChatCompletionInput,
} from '@/lib/tauri-commands'
import type { MissionGetStatusOutput } from '@/lib/tauri-commands/mission'

type ErrorRecord = Record<string, unknown>

function asRecord(input: unknown): ErrorRecord | undefined {
  if (!input || typeof input !== 'object' || Array.isArray(input)) {
    return undefined
  }

  return input as ErrorRecord
}

function asString(input: unknown): string | undefined {
  if (typeof input !== 'string') {
    return undefined
  }

  const text = input.trim()
  return text || undefined
}

function asNumber(input: unknown): number | undefined {
  if (typeof input === 'number' && Number.isFinite(input)) {
    return input
  }

  if (typeof input === 'string') {
    const value = Number(input)
    if (Number.isFinite(value)) {
      return value
    }
  }

  return undefined
}

function truncateText(input: string, max = 4000): string {
  if (input.length <= max) {
    return input
  }

  return `${input.slice(0, max)}…<truncated>`
}

function normalizeBody(input: unknown): unknown {
  if (typeof input === 'string') {
    const text = input.trim()
    if (!text) {
      return ''
    }

    try {
      return JSON.parse(text)
    } catch {
      return truncateText(text)
    }
  }

  if (input == null) {
    return undefined
  }

  return input
}

function logCompletionError(error: unknown) {
  const record = asRecord(error)
  const details = asRecord(record?.details)
  const requestId = asString(details?.request_id)
    || asString(details?.requestId)
    || asString(record?.request_id)
  const summary = formatUnknownError(error, 'E_AGENT_COMPLETION_FAILED')

  console.error('[agent-chat] ai_openai_chat_completion failed', {
    summary,
    code: asString(record?.code) || asString(details?.code),
    status: asNumber(details?.status) || asNumber(record?.status),
    requestId,
    recoverable: typeof record?.recoverable === 'boolean' ? record.recoverable : undefined,
    body: normalizeBody(details?.body),
    details,
    rawError: error,
  })
}

export type AgentCompletionInput = OpenAiChatCompletionInput

export interface AgentTurnResumeInputFeature {
  session_id: string
  turn_id: number
  resume_input:
    | { kind: 'confirmation'; allowed: boolean }
    | { kind: 'askuser'; answers: unknown }
}

export async function runAgentCompletion(input: AgentCompletionInput) {
  try {
    return await aiOpenAiChatCompletionClient(input)
  } catch (error) {
    logCompletionError(error)
    throw error
  }
}

export async function loadAgentProviderSettings() {
  try {
    return await getOpenAiProviderSettingsClient()
  } catch {
    return getOpenAiProviderSettings()
  }
}

export async function saveAgentProviderSettings(input: {
  openai_base_url: string
  openai_api_key: string
  openai_model?: string
  openai_embedding_model?: string
  openai_embedding_base_url?: string
  openai_embedding_api_key?: string
  openai_local_embedding_base_url?: string
  openai_local_embedding_api_key?: string
  openai_embedding_source?: 'provider' | 'local'
  openai_embedding_enabled?: boolean
  openai_embedding_detected?: boolean
  openai_embedding_detection_reason?: string
  openai_enabled_models?: string[]
}) {
  try {
    return await saveOpenAiProviderSettingsClient(input)
  } catch {
    return saveOpenAiProviderSettings(input)
  }
}

export async function appendAgentHistoryEvent(
  projectPath: string,
  chapterId: string,
  event: unknown,
) {
  await appendChapterHistoryEvent(projectPath, chapterId, event)
}

export async function resumeAgentTurnFeature(input: AgentTurnResumeInputFeature): Promise<void> {
  await agentTurnResumeClient(input)
}

export async function missionStartFeature(input: MissionStartInput): Promise<void> {
  await missionStartCommand(input)
}

export async function missionCreateFeature(input: MissionCreateInput): Promise<MissionCreateOutput> {
  return missionCreateCommand(input)
}

export async function missionPauseFeature(projectPath: string, missionId: string): Promise<void> {
  await missionPauseCommand(projectPath, missionId)
}

export async function missionResumeFeature(projectPath: string, missionId: string): Promise<void> {
  await missionResumeCommand(projectPath, missionId)
}

export async function missionCancelFeature(projectPath: string, missionId: string): Promise<void> {
  await missionCancelCommand(projectPath, missionId)
}

export async function missionGetStatusFeature(
  projectPath: string,
  missionId: string,
): Promise<MissionGetStatusOutput> {
  return missionGetStatusCommand(projectPath, missionId)
}

export async function missionListFeature(projectPath: string): Promise<string[]> {
  return missionListCommand(projectPath)
}

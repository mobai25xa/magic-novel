import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import { AGENT_EVENT_CHANNEL, MISSION_EVENT_CHANNEL } from './channels'
import { dispatchAgentEvent } from './agent-dispatch'
import { dispatchMissionEvent } from './mission-dispatch'
import { resetMissionUiState } from './mission-store'
import type { AgentEventEnvelope, MissionEventEnvelope } from './types'

let unlistenAgent: UnlistenFn | null = null
let unlistenMission: UnlistenFn | null = null

/**
 * Start listening to Rust backend events.
 * Call once at app startup (or when agent engine v2 is enabled).
 * Returns a cleanup function.
 */
export async function startBackendEventListeners(): Promise<() => void> {
  // Prevent double-subscribe
  await stopBackendEventListeners()

  unlistenAgent = await listen<AgentEventEnvelope>(AGENT_EVENT_CHANNEL, (event) => {
    try {
      dispatchAgentEvent(event.payload)
    } catch (err) {
      console.error('[agent-event] dispatch error:', err)
    }
  })

  unlistenMission = await listen<MissionEventEnvelope>(MISSION_EVENT_CHANNEL, (event) => {
    try {
      dispatchMissionEvent(event.payload)
    } catch (err) {
      console.error('[mission-event] dispatch error:', err)
    }
  })

  return stopBackendEventListeners
}

/**
 * Stop listening to Rust backend events.
 */
export async function stopBackendEventListeners(): Promise<void> {
  if (unlistenAgent) {
    unlistenAgent()
    unlistenAgent = null
  }
  if (unlistenMission) {
    unlistenMission()
    unlistenMission = null
  }

  resetMissionUiState()
}


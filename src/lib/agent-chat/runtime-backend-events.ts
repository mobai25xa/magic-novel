/**
 * runtime-backend-events.ts
 *
 * Compatibility entrypoint.
 *
 * The implementation lives in `./runtime-backend-events/*` (modular runtime backend events).
 * Keep this file as the stable import path for the app.
 */

export type { MissionUiState } from './runtime-backend-events/types'
export {
  getMissionUiState,
  subscribeMissionUiState,
} from './runtime-backend-events/mission-store'
export {
  startBackendEventListeners,
  stopBackendEventListeners,
} from './runtime-backend-events/listeners'


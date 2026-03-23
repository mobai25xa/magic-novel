import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import { createTsModuleLoader } from './_ts-test-loader.mjs'

function createCheck(name, pass, detail) {
  return detail ? { name, pass, detail } : { name, pass }
}

async function main() {
  const root = resolve(import.meta.dirname, '..')
  const loadTs = createTsModuleLoader(root)
  const checks = []

  const eventBuilders = loadTs.loadModule(resolve(root, 'src/lib/agent-chat/session/session-event-builders.ts'))
  const storeRuntime = loadTs.loadModule(resolve(root, 'src/lib/agent-chat/session/store/session-store-runtime.ts'))
  const storeActions = loadTs.loadModule(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders.ts'))

  const messageEvent = eventBuilders.toSessionMessageEvent({
    sessionId: 'obs_session',
    message: {
      id: 'obs_msg_1',
      role: 'user',
      content: 'ping',
      turn: 2,
      ts: 1000,
    },
    diagnostics: {
      client_request_id: 'req_obs_1',
      hydrate_source: 'snapshot_loaded',
    },
  })

  const finalEvent = eventBuilders.toSessionTurnFinalEvent({
    sessionId: 'obs_session',
    turnId: 2,
    stopReason: 'success',
    diagnostics: {
      client_request_id: 'req_obs_1',
      hydrate_source: 'snapshot_loaded',
    },
  })

  checks.push(createCheck(
    'event_diagnostics_payloads',
    messageEvent.payload?.bound_turn_id === 2
      && messageEvent.payload?.client_request_id === 'req_obs_1'
      && messageEvent.payload?.hydrate_source === 'snapshot_loaded'
      && finalEvent.payload?.bound_turn_id === 2
      && finalEvent.payload?.client_request_id === 'req_obs_1',
  ))

  let state = storeRuntime.createInitialSessionStorePatch({ sessionId: 'obs_store' })
  const set = (next) => {
    state = {
      ...state,
      ...next,
    }
  }

  const applyHydration = storeActions.createApplySessionHydrationAction(set)
  applyHydration({
    sessionId: 'obs_store',
    hydrationStatus: 'snapshot_loaded',
    runtimeState: 'ready',
    canContinue: true,
    canResume: false,
    warnings: [],
    lastTurn: 4,
    nextTurnId: 5,
    sessionRevision: 12,
    hydrationSource: 'snapshot_loaded',
  })

  checks.push(createCheck(
    'hydration_observability_state',
    state.sessionLastTurn === 4
      && state.sessionNextTurnId === 5
      && state.sessionRevision === 12
      && state.sessionHydrationSource === 'snapshot_loaded',
  ))

  const files = {
    sessionIndex: readFileSync(resolve(root, 'src/lib/agent-chat/session/index.ts'), 'utf-8'),
    sessionClient: readFileSync(resolve(root, 'src/lib/agent-chat/session/session-client.ts'), 'utf-8'),
    backendSessionCommand: readFileSync(resolve(root, 'src-tauri/src/application/command_usecases/agent_session.rs'), 'utf-8'),
    loopEngine: readFileSync(resolve(root, 'src-tauri/src/agent_engine/loop_engine.rs'), 'utf-8'),
    toolRouting: readFileSync(resolve(root, 'src-tauri/src/agent_engine/tool_routing.rs'), 'utf-8'),
    createAction: readFileSync(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-create.ts'), 'utf-8'),
    loadAction: readFileSync(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-load.ts'), 'utf-8'),
    resumeAction: readFileSync(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-resume.ts'), 'utf-8'),
    runtime: readFileSync(resolve(root, 'src/lib/agent-chat/runtime.ts'), 'utf-8'),
    runtimeBackendEvents: [
      readFileSync(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/tool.ts'), 'utf-8'),
      readFileSync(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/turn.ts'), 'utf-8'),
      readFileSync(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/turn-failed.ts'), 'utf-8'),
      readFileSync(resolve(root, 'src/lib/agent-chat/runtime-backend-events/utils.ts'), 'utf-8'),
    ].join('\n'),
  }

  checks.push(createCheck(
    'frontend_session_single_path_no_append_queue',
    !files.sessionIndex.includes("export * from './session-persistence-runtime'")
      && !files.runtime.includes('queueMessagePersistence')
      && !files.runtimeBackendEvents.includes('queueTodoStatePersistence')
      && !files.runtimeBackendEvents.includes('queueToolTracePersistence'),
  ))

  checks.push(createCheck(
    'session_observability_metrics_present',
    files.createAction.includes('session_create_success_count')
      && files.createAction.includes('session_create_error_count')
      && files.loadAction.includes('session_list_load_success_count')
      && files.loadAction.includes('session_list_load_error_count')
      && files.resumeAction.includes('session_load_success_count')
      && files.resumeAction.includes('session_hydrate_success_count')
      && files.resumeAction.includes('session_load_error_count')
      && files.resumeAction.includes('session_hydrate_error_count'),
  ))

  checks.push(createCheck(
    'append_error_rate_metrics_frontend_backend_aligned',
    files.sessionClient.includes('agent_session_append_events_success_count')
      && files.sessionClient.includes('agent_session_append_events_error_count')
      && files.backendSessionCommand.includes('metric = "agent_session_append_events_success_count"')
      && files.backendSessionCommand.includes('metric = "agent_session_append_events_error_count"'),
  ))

  checks.push(createCheck(
    'todo_state_remains_live_without_frontend_persistence_append',
    files.runtimeBackendEvents.includes('normalizeTodoStateFromToolResultPayload')
      && files.runtimeBackendEvents.includes('store.applyTodoState(todoState)')
      && !files.runtimeBackendEvents.includes('queueTodoStatePersistence({'),
  ))

  checks.push(createCheck(
    'tool_exposure_telemetry_visible_in_runtime',
    files.runtimeBackendEvents.includes("case 'PLAN_STARTED'")
      && files.runtimeBackendEvents.includes('extractToolExposureMeta(payload)')
      && files.runtimeBackendEvents.includes('tool_package'),
  ))

  checks.push(createCheck(
    'phase4_rollout_and_observability_metrics_present',
    files.toolRouting.includes('MAGIC_TOOL_PACKAGE_ROLLOUT_MODE')
      && files.toolRouting.includes('rollout_in_canary')
      && files.loopEngine.includes('tool_schema_reject_count')
      && files.loopEngine.includes('provider_400_count')
      && files.loopEngine.includes('turn_failed_before_first_token_count')
      && files.loopEngine.includes('package_fallback_rate')
      && files.loopEngine.includes('missing_tool_escalation_rate')
      && files.runtimeBackendEvents.includes('turn_failed_classification')
      && files.runtimeBackendEvents.includes('rollout_mode'),
  ))

  checks.push(createCheck(
    'resume_reminder_observability',
    files.resumeAction.includes('next_turn_id: loaded.hydration.nextTurnId ?? null')
      && files.resumeAction.includes('hydration_source: loaded.hydration.hydrationSource ?? loaded.hydration.hydrationStatus'),
  ))

  const allPass = checks.every((check) => check.pass)
  console.log('[dev-c-observability]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[dev-c-observability] failed:', error.message)
  process.exit(1)
})

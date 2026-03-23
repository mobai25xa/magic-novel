import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function extractSection(source, startMarker, endMarker) {
  const startIndex = source.indexOf(startMarker)
  if (startIndex === -1) {
    return ''
  }

  const afterStart = source.slice(startIndex)
  if (!endMarker) {
    return afterStart
  }

  const endIndex = afterStart.indexOf(endMarker)
  return endIndex === -1 ? afterStart : afterStart.slice(0, endIndex)
}

async function main() {
  const root = resolve(import.meta.dirname, '..')

  const files = {
    sessionState: await readFile(resolve(root, 'src-tauri/src/agent_engine/session_state.rs'), 'utf-8'),
    engineCommands: [
      await readFile(resolve(root, 'src-tauri/src/commands/agent_engine.rs'), 'utf-8'),
      await readFile(resolve(root, 'src-tauri/src/commands/agent_engine/core.rs'), 'utf-8'),
    ].join('\n'),
    sessionHydrate: await readFile(resolve(root, 'src-tauri/src/application/command_usecases/agent_session.rs'), 'utf-8'),
    runtimeSnapshot: await readFile(resolve(root, 'src-tauri/src/services/agent_session/runtime_snapshot.rs'), 'utf-8'),
    events: await readFile(resolve(root, 'src-tauri/src/agent_engine/events.rs'), 'utf-8'),
    emitter: await readFile(resolve(root, 'src-tauri/src/agent_engine/emitter.rs'), 'utf-8'),
    agentTypes: await readFile(resolve(root, 'src-tauri/src/agent_engine/types.rs'), 'utf-8'),
    runtime: await readFile(resolve(root, 'src/lib/agent-chat/runtime.ts'), 'utf-8'),
    runtimeTurnHandler: await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/turn.ts'), 'utf-8'),
    runtimeToolHandler: await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/tool.ts'), 'utf-8'),
    runtimeAskUserHandler: await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/askuser.ts'), 'utf-8'),
    turnCard: await readFile(resolve(root, 'src/components/ai/turn-card.tsx'), 'utf-8'),
    chatStore: await readFile(resolve(root, 'src/lib/agent-chat/store.ts'), 'utf-8'),
    engineClient: await readFile(resolve(root, 'src/platform/tauri/clients/agent-engine-client.ts'), 'utf-8'),
    sessionClient: await readFile(resolve(root, 'src/platform/tauri/clients/agent-session-client.ts'), 'utf-8'),
    sessionTests: await readFile(resolve(root, 'src-tauri/src/services/agent_session/tests.rs'), 'utf-8'),
    engineCommandTests: await readFile(resolve(root, 'src-tauri/src/commands/agent_engine/core/tests.rs'), 'utf-8'),
  }

  const waitingConfirmationCase = extractSection(
    files.runtimeToolHandler,
    'function handleWaitingForConfirmation',
    '',
  )
  const askUserRequestedCase = extractSection(
    files.runtimeAskUserHandler,
    'function handleAskUserRequested',
    'function handleAskUserAnswered',
  )
  const turnCompletedCase = extractSection(
    files.runtimeTurnHandler,
    'function handleTurnCompleted',
    'function handleTurnCancelled',
  )
  const resolveAskUserSection = extractSection(
    files.chatStore,
    'resolveAskUserRequest: (callId, answers) => {',
    'cancelAskUserRequest: (callId) => {',
  )

  const checks = [
    {
      name: 'session_state_manager_authority_contract',
      pass:
        files.sessionState.includes('pub fn derive_next_turn_id(') &&
        files.sessionState.includes('pub fn seed_next_turn_id(') &&
        files.sessionState.includes('pub fn save_runtime_state(') &&
        files.sessionState.includes('pub fn save_suspended_runtime_state(') &&
        files.sessionState.includes('pub fn with_session_turn_lock') &&
        files.sessionState.includes('test_with_session_turn_lock_serializes_seed_and_allocate'),
    },
    {
      name: 'runtime_snapshot_next_turn_contract',
      pass:
        files.runtimeSnapshot.includes('pub next_turn_id: Option<u32>') &&
        files.runtimeSnapshot.includes('pub next_turn_id: Option<u32>,') &&
        files.runtimeSnapshot.includes('derive_next_turn_id(') &&
        files.runtimeSnapshot.includes('runtime_snapshot_load_migrates_missing_next_turn_id'),
    },
    {
      name: 'hydrate_output_authoritative_cursor_contract',
      pass:
        files.sessionHydrate.includes('pub next_turn_id: Option<u32>') &&
        files.sessionHydrate.includes('pub hydration_source: Option<String>') &&
        files.sessionHydrate.includes('pub session_revision: Option<u64>') &&
        files.sessionHydrate.includes('derive_runtime_next_turn_id(') &&
        files.sessionHydrate.includes('save_runtime_state(') &&
        files.sessionHydrate.includes('save_suspended_runtime_state(') &&
        files.sessionHydrate.includes('next_turn_id: Some(next_turn_id)') &&
        files.sessionHydrate.includes('hydration_source:'),
    },
    {
      name: 'turn_start_hydrate_then_allocate_contract',
      pass:
        files.engineCommands.includes('pub client_request_id: Option<String>') &&
        files.engineCommands.includes('prepare_turn_start(') &&
        files.engineCommands.includes('seed_next_turn_id(session_id, hydrated.next_turn_id)') &&
        files.engineCommands.includes('let turn_id = session_state::global().next_turn_id(session_id);') &&
        files.engineCommands.includes('.with_client_request_id(Some(client_request_id.clone()))') &&
        files.engineCommands.includes('hydration_status: Some(hydration_status)'),
    },
    {
      name: 'resume_reuses_suspended_turn_contract',
      pass:
        files.engineCommands.includes('prepare_resume_turn(&input.session_id)?') &&
        files.engineCommands.includes('let turn_id = suspended.conversation_state.current_turn;') &&
        files.engineCommands.includes('authoritative_turn_id') &&
        files.engineCommandTests.includes('prepare_resume_turn_reuses_suspended_turn_id'),
    },
    {
      name: 'client_request_id_event_contract',
      pass:
        files.agentTypes.includes('pub client_request_id: String') &&
        files.events.includes('pub client_request_id: Option<String>') &&
        files.events.includes('new_with_client_request_id(') &&
        files.emitter.includes('with_client_request_id(') &&
        !files.emitter.includes('attach_client_request_id(') &&
        files.engineClient.includes('client_request_id: string'),
    },
    {
      name: 'frontend_binding_uses_authoritative_turn',
      pass:
        files.runtime.includes('createClientRequestId') &&
        files.runtime.includes('client_request_id: clientRequestId') &&
        files.runtime.includes('bindAuthoritativeTurn({') &&
        files.runtime.includes('startAck.client_request_id ?? clientRequestId'),
    },
    {
      name: 'resume_unlock_waits_for_turn_completed',
      pass:
        !waitingConfirmationCase.includes("runtimeState: 'suspended_confirmation'") &&
        !waitingConfirmationCase.includes('canResume: true') &&
        !askUserRequestedCase.includes("runtimeState: 'suspended_askuser'") &&
        !askUserRequestedCase.includes('canResume: true') &&
        turnCompletedCase.includes("rawStopReason === 'waiting_confirmation'") &&
        turnCompletedCase.includes("runtimeState: 'suspended_confirmation'") &&
        turnCompletedCase.includes("rawStopReason === 'waiting_askuser'") &&
        turnCompletedCase.includes("runtimeState: 'suspended_askuser'") &&
        turnCompletedCase.includes('canResume: true') &&
        files.turnCard.includes('const allowResumeAction = input.sessionCanResume') &&
        files.turnCard.includes("input.sessionRuntimeState === 'suspended_confirmation' || input.sessionRuntimeState === 'suspended_askuser'") &&
        files.turnCard.includes('pendingAskUser={allowResumeAction ? turnPendingAskUser : undefined}'),
    },
    {
      name: 'askuser_resume_failure_restores_pending_ui',
      pass:
        resolveAskUserSection.includes("resume (askuser) failed") &&
        resolveAskUserSection.includes('pendingAskUser: pending') &&
        resolveAskUserSection.includes("sessionRuntimeState: 'suspended_askuser'") &&
        resolveAskUserSection.includes('sessionCanResume: true') &&
        resolveAskUserSection.includes('reduceMarkWaitingForConfirmation('),
    },
    {
      name: 'hydrate_tests_cover_cursor_restore',
      pass:
        files.sessionTests.includes('test_hydrate_memory_hit_from_in_memory_conversation') &&
        files.sessionTests.includes('test_hydrate_snapshot_loaded_for_completed_session') &&
        files.sessionTests.includes('test_hydrate_readonly_fallback_for_suspended_without_snapshot') &&
        files.sessionClient.includes('next_turn_id?: number'),
    },
  ]

  const allPass = checks.every((check) => check.pass)
  console.log('[test-dev-a-runtime-state-machine]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[test-dev-a-runtime-state-machine] failed:', error.message)
  process.exit(1)
})

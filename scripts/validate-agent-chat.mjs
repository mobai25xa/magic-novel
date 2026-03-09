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

  const replayTests = loadTs.loadModule(resolve(root, 'src/lib/agent-chat/session/__tests__/session-replay-tests.ts'))
  if (typeof replayTests.runAll !== 'function') {
    throw new Error('session-replay-tests.ts does not export runAll()')
  }
  replayTests.runAll()
  checks.push(createCheck('session_replay_contract_tests', true))

  const files = {
    tauriSessionClient: readFileSync(resolve(root, 'src/platform/tauri/clients/agent-session-client.ts'), 'utf-8'),
    sessionClient: readFileSync(resolve(root, 'src/lib/agent-chat/session/session-client.ts'), 'utf-8'),
    sessionHydration: readFileSync(resolve(root, 'src/lib/agent-chat/session/session-hydration.ts'), 'utf-8'),
    sessionControllerOps: readFileSync(resolve(root, 'src/lib/agent-chat/session/session-controller-ops.ts'), 'utf-8'),
    sessionReducer: readFileSync(resolve(root, 'src/lib/agent-chat/session/session-reducer.ts'), 'utf-8'),
    sessionStoreContract: readFileSync(resolve(root, 'src/lib/agent-chat/session/store/session-store-contract.ts'), 'utf-8'),
    sessionStoreActions: readFileSync(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders.ts'), 'utf-8'),
    sessionEventBuilders: readFileSync(resolve(root, 'src/lib/agent-chat/session/session-event-builders.ts'), 'utf-8'),
    chatStore: readFileSync(resolve(root, 'src/lib/agent-chat/store.ts'), 'utf-8'),
    layoutStore: readFileSync(resolve(root, 'src/stores/layout-store.ts'), 'utf-8'),
    editorPage: readFileSync(resolve(root, 'src/components/editor/EditorPage.tsx'), 'utf-8'),
  }

  checks.push(createCheck(
    'hydrate_authority_fields_wired',
    files.tauriSessionClient.includes('next_turn_id?: number')
      && files.tauriSessionClient.includes('session_revision?: number')
      && files.tauriSessionClient.includes('hydration_source?: string')
      && files.sessionClient.includes('nextTurnId: hydrated.next_turn_id')
      && files.sessionClient.includes('sessionRevision: hydrated.session_revision')
      && files.sessionClient.includes('hydrationSource: hydrated.hydration_source')
      && files.sessionHydration.includes('normalizeSessionHydration'),
  ))

  checks.push(createCheck(
    'replay_and_authority_split_wired',
    files.sessionReducer.includes('replayTurn: replay.turn')
      && files.sessionStoreContract.includes('sessionReplayTurn: number')
      && files.sessionStoreContract.includes('sessionNextTurnId?: number')
      && files.sessionStoreActions.includes('sessionReplayTurn: patch.replayTurn')
      && files.sessionStoreActions.includes('sessionNextTurnId: input.nextTurnId'),
  ))

  checks.push(createCheck(
    'history_fallback_normalization_wired',
    files.sessionControllerOps.includes('inferHistoricalLastTurn')
      && files.sessionControllerOps.includes('normalizeSessionHydration')
      && files.sessionControllerOps.includes('runtime_snapshot_rebuilt_from_event_log'),
  ))

  checks.push(createCheck(
    'session_event_diagnostics_wired',
    files.sessionEventBuilders.includes('bound_turn_id')
      && files.sessionEventBuilders.includes('client_request_id')
      && files.sessionEventBuilders.includes('hydrate_source'),
  ))

  checks.push(createCheck(
    'pending_cancel_cleans_pending_ui',
    files.chatStore.includes("requestPendingTurnCancellation: (clientRequestId) =>")
      && files.chatStore.includes("pendingRequest.status === 'cancel_requested'")
      && files.chatStore.includes("state.messages.filter((message) => !pendingMessageIdSet.has(message.id))"),
  ))

  checks.push(createCheck(
    'active_session_blocks_right_panel_autohide',
    files.layoutStore.includes('disableRightPanelAutoHide?: boolean')
      && files.layoutStore.includes('!disableRightPanelAutoHide && state.isRightPanelVisible')
      && files.editorPage.includes("sessionRuntimeState === 'running'")
      && files.editorPage.includes("sessionRuntimeState === 'suspended_confirmation'")
      && files.editorPage.includes("sessionRuntimeState === 'suspended_askuser'")
      && files.editorPage.includes('useEditorPanelAutoHide({ disableRightPanelAutoHide: isAgentSessionActive })'),
  ))

  const allPass = checks.every((check) => check.pass)
  console.log('[validate-agent-chat]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[validate-agent-chat] failed:', error.message)
  process.exit(1)
})

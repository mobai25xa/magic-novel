import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function has(content, ...needles) {
  return needles.every((needle) => content.includes(needle))
}

async function main() {
  const root = resolve(import.meta.dirname, '..')

  const files = {
    agentSessionClient: await readFile(resolve(root, 'src/platform/tauri/clients/agent-session-client.ts'), 'utf-8'),
    sessionClient: await readFile(resolve(root, 'src/lib/agent-chat/session/session-client.ts'), 'utf-8'),
    sessionController: await readFile(resolve(root, 'src/lib/agent-chat/session/session-controller.ts'), 'utf-8'),
    sessionControllerOps: await readFile(resolve(root, 'src/lib/agent-chat/session/session-controller-ops.ts'), 'utf-8'),
    createAction: await readFile(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-create.ts'), 'utf-8'),
    loadAction: await readFile(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-load.ts'), 'utf-8'),
    resumeAction: await readFile(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-resume.ts'), 'utf-8'),
    deleteAction: await readFile(resolve(root, 'src/lib/agent-chat/session/store/session-store-action-builders-delete.ts'), 'utf-8'),
    runtime: await readFile(resolve(root, 'src/lib/agent-chat/runtime.ts'), 'utf-8'),
    runtimeBackendEvents: await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events.ts'), 'utf-8'),
    sessionIndex: await readFile(resolve(root, 'src/lib/agent-chat/session/index.ts'), 'utf-8'),
  }

  const checks = [
    {
      name: 'session_crud_and_hydrate_commands_exist',
      pass: has(
        files.agentSessionClient,
        "invokeTauri('agent_session_create'",
        "invokeTauri('agent_session_list'",
        "invokeTauri('agent_session_load'",
        "invokeTauri('agent_session_hydrate'",
        "invokeTauri('agent_session_update_meta'",
        "invokeTauri('agent_session_delete'",
      ),
    },
    {
      name: 'session_client_wires_create_list_load_hydrate_rename_delete',
      pass: has(
        files.sessionClient,
        'createPersistedSessionClient',
        'listPersistedSessionsClient',
        'loadPersistedSessionClient',
        'hydratePersistedSessionClient',
        'renamePersistedSessionClient',
        'deletePersistedSessionClient',
      ),
    },
    {
      name: 'controller_resume_uses_load_then_hydrate',
      pass: has(
        files.sessionControllerOps,
        'loadSessionForResume',
        'hydrateSessionForResume',
        'fallbackHydrationFromHistory',
        'resumePersistedSession',
      ),
    },
    {
      name: 'session_controller_no_queue_flush_api_surface',
      pass: !files.sessionController.includes('queuePersistedSessionEvent')
        && !files.sessionController.includes('queuePersistedSessionEventWithoutProject')
        && !files.sessionController.includes('flushPersistedSessionEvents')
        && !files.sessionControllerOps.includes('queuePersistedSessionEvent(')
        && !files.sessionControllerOps.includes('queuePersistedSessionEventWithoutProject(')
        && !files.sessionControllerOps.includes('flushPersistedSessionEvents(')
        && !files.sessionControllerOps.includes('completePersistedTurn('),
    },
    {
      name: 'append_metrics_exist_for_error_rate_closure',
      pass: files.sessionClient.includes('agent_session_append_events_success_count')
        && files.sessionClient.includes('agent_session_append_events_error_count'),
    },
    {
      name: 'store_actions_cover_create_list_resume_rename_delete',
      pass: has(files.createAction, 'createNewPersistedSession')
        && has(files.loadAction, 'loadPersistedSessions')
        && has(files.resumeAction, 'resumePersistedSession')
        && has(files.deleteAction, 'renamePersistedSession', 'deletePersistedSession'),
    },
    {
      name: 'frontend_runtime_no_session_append_queue_path',
      pass: !files.sessionIndex.includes("export * from './session-persistence-runtime'")
        && !files.runtime.includes('queueMessagePersistence')
        && !files.runtimeBackendEvents.includes('queueTodoStatePersistence')
        && !files.runtimeBackendEvents.includes('queueToolTracePersistence'),
    },
  ]

  const allPass = checks.every((check) => check.pass)
  console.log('[test-session-regression]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[test-session-regression] failed:', error.message)
  process.exit(1)
})

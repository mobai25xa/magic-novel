import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function has(content, ...needles) {
  return needles.every((needle) => content.includes(needle))
}

async function main() {
  const root = resolve(import.meta.dirname, '..')

  const files = {
    versioningClient: await readFile(resolve(root, 'src/platform/tauri/clients/agent-versioning-client.ts'), 'utf-8'),
    versioningCommands: await readFile(resolve(root, 'src/lib/tauri-commands/agent-versioning.ts'), 'utf-8'),
    sessionClient: await readFile(resolve(root, 'src/platform/tauri/clients/agent-session-client.ts'), 'utf-8'),
    sessionCommands: await readFile(resolve(root, 'src/lib/tauri-commands/agent-session.ts'), 'utf-8'),
  }

  const checks = [
    {
      name: 'rollback_clients_exposed',
      pass: has(
        files.versioningClient,
        "invokeTauri('vc_rollback_by_revision'",
        "invokeTauri('vc_rollback_by_call_id'",
        "invokeTauri('vc_recover'",
      ),
    },
    {
      name: 'rollback_commands_exposed',
      pass: has(
        files.versioningCommands,
        'export async function vcRollbackByRevision',
        'export async function vcRollbackByCallId',
        'export async function vcRecover',
      ),
    },
    {
      name: 'session_recover_exposed',
      pass: has(
        files.sessionClient,
        "invokeTauri('agent_session_recover'",
      ) && has(
        files.sessionCommands,
        'export async function agentSessionRecover',
      ),
    },
  ]

  const allPass = checks.every((item) => item.pass)

  console.log('[session-v2-rollback]', JSON.stringify({
    all_pass: allPass,
    checks,
  }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[session-v2-rollback] failed:', error.message)
  process.exit(1)
})

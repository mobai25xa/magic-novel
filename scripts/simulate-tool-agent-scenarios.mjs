import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function has(content, ...needles) {
  return needles.every((needle) => content.includes(needle))
}

async function main() {
  const projectRoot = resolve(import.meta.dirname, '..')
  const typesPath = resolve(projectRoot, 'src/lib/tool-gateway/types.ts')
  const runtimeClientPath = resolve(projectRoot, 'src/platform/tauri/clients/tool-runtime-client.ts')
  const runtimePath = resolve(projectRoot, 'src/lib/agent-test/runtime.ts')

  const [typesCode, runtimeClientCode, runtimeCode] = await Promise.all([
    readFile(typesPath, 'utf-8'),
    readFile(runtimeClientPath, 'utf-8'),
    readFile(runtimePath, 'utf-8'),
  ])

  const checks = [
    {
      name: 'create_volume',
      pass: has(
        typesCode,
        "kind: 'volume'",
      ) && has(runtimeClientCode, 'kind: input.kind'),
      reason: 'volume creation canonical contract is declared and forwarded',
    },
    {
      name: 'create_chapter_in_explicit_volume',
      pass: has(
        typesCode,
        "kind: 'chapter'",
        'volume_path: string',
      ) && has(runtimeClientCode, 'volume_path: isChapter ? input.volume_path : undefined'),
      reason: 'chapter create requires explicit volume path and forwards it',
    },
    {
      name: 'read_volume_meta',
      pass: has(
        typesCode,
        "kind: 'volume'",
        "view: 'meta'",
      ) && has(runtimeClientCode, 'kind: input.kind', 'view: input.view'),
      reason: 'volume read is constrained to meta and mapped to backend payload',
    },
    {
      name: 'edit_chapter_meta',
      pass: has(
        typesCode,
        "target: 'chapter_meta'",
      ) && has(
        runtimeClientCode,
        "target: input.target",
        "status: 'status' in input ? input.status : undefined",
        "target_words: 'target_words' in input ? input.target_words : undefined",
        "tags: 'tags' in input ? input.tags : undefined",
      ),
      reason: 'chapter meta edit target and fields are modeled and mapped',
    },
    {
      name: 'delete_chapter_preview',
      pass: has(
        typesCode,
        "kind: 'volume' | 'chapter'",
        'dry_run?: boolean',
      ) && has(
        runtimeClientCode,
        'export async function runtimeToolDelete',
        "invokeTauri<ToolResult<AnyRecord>>('tool_delete'",
        "mode: asString(res.data.mode) === 'commit' ? 'commit' : 'preview'",
      ),
      reason: 'delete preview path is represented and result mode is preserved',
    },
    {
      name: 'delete_volume_preview',
      pass: has(
        runtimeClientCode,
        'kind: input.kind',
        'impact: asRecord(res.data.impact)',
      ),
      reason: 'volume delete preview/commit impact mapping is preserved',
    },
    {
      name: 'move_chapter_between_volumes',
      pass: has(
        typesCode,
        'chapter_path: string',
        'target_volume_path: string',
        'target_index: number',
      ) && has(
        runtimeClientCode,
        'export async function runtimeToolMove',
        "invokeTauri<ToolResult<AnyRecord>>('tool_move'",
        'new_chapter_path: asOptionalString(res.data.new_chapter_path)',
      ),
      reason: 'move payload and response mapping cover cross-volume relocation',
    },
    {
      name: 'agent_test_runtime_uses_canonical_shape',
      pass: has(
        runtimeCode,
        "kind: 'chapter'",
        "target: 'chapter_content'",
      ),
      reason: 'agent test runtime call sites switched to canonical fields',
    },
  ]

  const allPass = checks.every((c) => c.pass)
  const summary = {
    all_pass: allPass,
    checks: checks.map((c) => ({ name: c.name, pass: c.pass, reason: c.reason })),
  }

  console.log('[tool-agent-scenarios]', JSON.stringify(summary))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[tool-agent-scenarios] failed:', error.message)
  process.exit(1)
})

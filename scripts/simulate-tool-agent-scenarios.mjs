import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function has(content, ...needles) {
  return needles.every((needle) => content.includes(needle))
}

function lacks(content, ...needles) {
  return needles.every((needle) => !content.includes(needle))
}

async function main() {
  const projectRoot = resolve(import.meta.dirname, '..')
  const typesPath = resolve(projectRoot, 'src/lib/tool-gateway/types.ts')
  const gatewayPath = resolve(projectRoot, 'src/lib/tool-gateway/gateway.ts')
  const runtimeClientPath = resolve(projectRoot, 'src/platform/tauri/clients/tool-runtime-client.ts')
  const runtimePath = resolve(projectRoot, 'src/lib/agent-test/runtime.ts')
  const scenarioV2Path = resolve(projectRoot, 'src/lib/agent-test/tool-agent-scenario-v2.ts')
  const workerToolContractPath = resolve(projectRoot, 'src/features/global-config/worker-tool-contract.ts')
  const workerManagementPath = resolve(projectRoot, 'src/components/workers/worker-management.ts')

  const [typesCode, gatewayCode, runtimeClientCode, runtimeCode, scenarioV2Code, workerToolContractCode, workerManagementCode] = await Promise.all([
    readFile(typesPath, 'utf-8'),
    readFile(gatewayPath, 'utf-8'),
    readFile(runtimeClientPath, 'utf-8'),
    readFile(runtimePath, 'utf-8'),
    readFile(scenarioV2Path, 'utf-8'),
    readFile(workerToolContractPath, 'utf-8'),
    readFile(workerManagementPath, 'utf-8'),
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
    {
      name: 'gateway_interface_and_runtime_gateway_share_same_callable_surface',
      pass: has(
        typesCode,
        'create(input: ToolCreateInput)',
        'read(input: ToolReadInput)',
        'edit(input: ToolEditInput)',
        'delete(input: ToolDeleteInput)',
        'move(input: ToolMoveInput)',
        'ls(input: ToolLsInput)',
        'grep(input: ToolGrepInput)',
      ) && has(
        gatewayCode,
        'class RuntimeToolGateway implements ToolGateway',
        'create(input: ToolCreateInput)',
        'read(input: ToolReadInput)',
        'edit(input: ToolEditInput)',
        'delete(input: ToolDeleteInput)',
        'move(input: ToolMoveInput)',
        'ls(input: ToolLsInput)',
        'grep(input: ToolGrepInput)',
      ),
      reason: 'runtime gateway only exposes the canonical callable tool surface',
    },
    {
      name: 'tool_gateway_names_are_scoped_to_runtime_gateway_only',
      pass: has(
        typesCode,
        'export const TOOL_GATEWAY_NAMES = [',
        "'create'",
        "'read'",
        "'edit'",
        "'delete'",
        "'move'",
        "'ls'",
        "'grep'",
        'export type ToolGatewayName = typeof TOOL_GATEWAY_NAMES[number]',
        'tool: ToolGatewayName',
      )
        && lacks(
          typesCode,
          "'askuser'",
          "'outline'",
          "'character_sheet'",
          "'search_knowledge'",
        ),
      reason: 'tool-gateway types only own the runtime callable create/read/edit/delete/move/ls/grep surface',
    },
    {
      name: 'scenario_v2_uses_current_gateway_tools_only',
      pass: has(
        scenarioV2Code,
        "'ls'",
        "'read'",
        "'grep'",
        "'edit'",
        'toolGateway.ls({',
        'toolGateway.read({',
        'toolGateway.grep({',
        'toolGateway.edit({',
        "target: 'chapter_content'",
        "view: 'snapshot'",
      )
        && lacks(
          scenarioV2Code,
          'workspace_map',
          'context_read',
          'context_search',
          'draft_write',
          'knowledge_write',
          'knowledge_read',
      ),
      reason: 'scenario v2 is pinned to the current ls/read/grep/edit gateway contract',
    },
    {
      name: 'builtin_worker_tools_are_owned_by_global_config_contract',
      pass: has(
        workerToolContractCode,
        'export const BUILTIN_WORKER_TOOL_NAMES = [',
        "'workspace_map'",
        "'context_read'",
        "'context_search'",
        "'knowledge_read'",
        "'knowledge_write'",
        "'draft_write'",
        "'structure_edit'",
        "'review_check'",
        "'skill'",
        "'todowrite'",
        'export const DEFAULT_WORKER_TOOL_WHITELIST: string[] = [...BUILTIN_WORKER_TOOL_NAMES]',
      )
        && has(
          workerManagementCode,
          'BUILTIN_WORKER_TOOL_NAMES',
          'DEFAULT_WORKER_TOOL_WHITELIST',
          'export const AVAILABLE_WORKER_TOOLS = BUILTIN_WORKER_TOOL_NAMES',
          'tool_whitelist: [...DEFAULT_WORKER_TOOL_WHITELIST]',
        ),
      reason: 'worker-management uses a shared worker tool contract instead of duplicating agent tool ids inline',
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

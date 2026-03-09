import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function has(content, ...needles) {
  return needles.every((needle) => content.includes(needle))
}

async function main() {
  const root = resolve(import.meta.dirname, '..')

  const files = {
    types: await readFile(resolve(root, 'src/lib/tool-gateway/types.ts'), 'utf-8'),
    gateway: await readFile(resolve(root, 'src/lib/tool-gateway/gateway.ts'), 'utf-8'),
    runtimeClient: await readFile(resolve(root, 'src/platform/tauri/clients/tool-runtime-client.ts'), 'utf-8'),
    toolRegistry: await readFile(resolve(root, 'src-tauri/src/agent_tools/registry/mod.rs'), 'utf-8'),
    writingRegistry: await readFile(resolve(root, 'src-tauri/src/agent_tools/registry/writing.rs'), 'utf-8'),
  }

  const checks = [
    {
      name: 'tool_names_include_delete_move',
      pass: has(files.types, "| 'delete'", "| 'move'"),
    },
    {
      name: 'types_declare_delete_move_contracts',
      pass: has(
        files.types,
        'export interface ToolDeleteInput',
        'export interface ToolDeleteOutput',
        'export interface ToolMoveInput',
        'export interface ToolMoveOutput',
      ),
    },
    {
      name: 'gateway_exposes_delete_move',
      pass: has(
        files.gateway,
        'runtimeToolDelete',
        'runtimeToolMove',
        'delete(input: ToolDeleteInput)',
        'move(input: ToolMoveInput)',
      ),
    },
    {
      name: 'runtime_client_exports_delete_move',
      pass: has(
        files.runtimeClient,
        'export async function runtimeToolDelete',
        'export async function runtimeToolMove',
        "invokeTauri<ToolResult<AnyRecord>>('tool_delete'",
        "invokeTauri<ToolResult<AnyRecord>>('tool_move'",
      ),
    },
    {
      name: 'runtime_client_drops_legacy_normalizers',
      pass:
        !files.runtimeClient.includes('normalizeCreateInput(')
        && !files.runtimeClient.includes('normalizeReadInput(')
        && !files.runtimeClient.includes('normalizeEditInput(')
        && !files.runtimeClient.includes('resolveEditContent('),
    },
    {
      name: 'runtime_client_uses_v2_payload_fields',
      pass: has(
        files.runtimeClient,
        'kind: input.kind',
        'view: input.view',
        'snapshot_id: input.target === \'chapter_content\' ? input.snapshot_id : undefined',
        'ops: input.target === \'chapter_content\' ? input.ops : undefined',
      ),
    },
    {
      name: 'runtime_client_maps_delete_move_results',
      pass: has(
        files.runtimeClient,
        'function mapDeleteResult',
        'function mapMoveResult',
        'tx_id: asOptionalString(res.data.tx_id)',
      ),
    },
    {
      name: 'provider_safe_edit_schema_contract',
      pass:
        has(
          files.writingRegistry,
          '"enum": ["volume_meta", "chapter_meta", "chapter_content"]',
          '"required": ["target", "path"]',
          '"required": ["op"]',
        )
        && !files.writingRegistry.includes('"oneOf": [')
        && !files.writingRegistry.includes('"const": "chapter_content"')
        && !files.writingRegistry.includes('"const": "volume_meta"')
        && !files.writingRegistry.includes('"const": "chapter_meta"'),
    },
    {
      name: 'tool_registry_validates_provider_schema_safety',
      pass: has(
        files.toolRegistry,
        'fn validate_provider_parameters_schema(',
        'fn validate_provider_parameters_schema_node(',
        'skipping provider-incompatible tool schema',
      ),
    },
  ]

  const allPass = checks.every((check) => check.pass)
  console.log('[tool-agent-tests]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[tool-agent-tests] failed:', error.message)
  process.exit(1)
})

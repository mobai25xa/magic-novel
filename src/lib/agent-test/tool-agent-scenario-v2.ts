import { toolGateway } from '@/lib/tool-gateway/gateway'
import { createCallId } from '@/lib/tool-gateway/utils'
import type { ToolGatewayName } from '@/lib/tool-gateway/types'

const SCENARIO_TOOL_SEQUENCE: ToolGatewayName[] = [
  'ls',
  'read',
  'grep',
  'edit',
]

const SCENARIO_MARKDOWN = '# Scenario V2\n\n这是一次测试修改。'

function normalizeChapterPath(path: string) {
  return path.trim().replace(/\\/g, '/').replace(/^chapter:/, '').replace(/^\/+/, '')
}

export function getToolAgentScenarioV2Sequence(): ToolGatewayName[] {
  return [...SCENARIO_TOOL_SEQUENCE]
}

export async function runToolAgentScenarioV2(input: {
  projectPath: string
  chapterRef: string
}): Promise<ToolGatewayName[]> {
  const { projectPath, chapterRef } = input
  const chapterPath = normalizeChapterPath(chapterRef)

  await toolGateway.ls({
    project_path: projectPath,
    path: '.',
    limit: 20,
    call_id: createCallId('ls'),
  })

  const readResult = await toolGateway.read({
    project_path: projectPath,
    kind: 'chapter',
    path: chapterPath,
    view: 'snapshot',
    call_id: createCallId('read'),
  })

  if (!readResult.ok || !readResult.data?.snapshot?.snapshot_id) {
    throw new Error(`tool scenario read failed for ${chapterPath}`)
  }

  const firstBlockId = readResult.data.snapshot.blocks.find(
    (block) => typeof block.block_id === 'string' && block.block_id.trim().length > 0,
  )?.block_id

  await toolGateway.grep({
    project_path: projectPath,
    query: '人物名',
    mode: 'keyword',
    scope: { paths: [chapterPath] },
    top_k: 5,
    call_id: createCallId('grep'),
  })

  await toolGateway.edit({
    project_path: projectPath,
    path: chapterPath,
    target: 'chapter_content',
    base_revision: readResult.data.revision,
    snapshot_id: readResult.data.snapshot.snapshot_id,
    ops: firstBlockId
      ? [{ op: 'replace_block', block_id: firstBlockId, markdown: SCENARIO_MARKDOWN }]
      : [{ op: 'append_blocks', blocks: [{ markdown: SCENARIO_MARKDOWN }] }],
    dry_run: true,
    actor: 'agent',
    call_id: createCallId('edit'),
  })

  return [...SCENARIO_TOOL_SEQUENCE]
}

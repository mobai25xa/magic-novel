import { toolGateway } from '@/lib/tool-gateway/gateway'
import { createCallId } from '@/lib/tool-gateway/utils'
import type { ToolResult } from '@/lib/tool-gateway/types'
import { useProjectStore } from '@/stores/project-store'

import { planAction } from './planner'
import { useAgentSessionStore } from './session-store'
import { traceError, traceSuccess } from './trace'

const TEST_EDIT_MARKDOWN = '# Agent Edit\n\n这是一次测试修改。'

export async function runAgentTurn(userInput: string): Promise<string> {
  const session = useAgentSessionStore.getState()
  const project = useProjectStore.getState()

  const turn = session.nextTurn()
  session.pushMessage({ role: 'user', content: userInput, ts: Date.now() })

  const planned = planAction(userInput)
  if (!planned) {
    const reply = '未识别意图，可用命令：create chapter/read snapshot/read json/preview/commit'
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  if (!project.projectPath) {
    const reply = '请先打开项目'
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  if (planned.tool === 'create') {
    return handleCreateTurn({
      turn,
      projectPath: project.projectPath,
      tree: project.tree,
      title: planned.title,
    })
  }

  const chapterPath = session.active_chapter_path || inferFirstChapterPath(project.tree)
  if (!chapterPath) {
    const reply = '未找到章节，请先 create chapter'
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  session.setActiveChapterPath(chapterPath)

  if (planned.tool === 'read') {
    return handleReadTurn({
      turn,
      projectPath: project.projectPath,
      chapterPath,
      view: planned.view,
    })
  }

  return handleEditTurn({
    turn,
    projectPath: project.projectPath,
    chapterPath,
    dryRun: planned.dry_run,
  })
}

type CreateTurnInput = {
  turn: number
  projectPath: string
  tree: unknown[]
  title: string
}

async function handleCreateTurn(input: CreateTurnInput): Promise<string> {
  const session = useAgentSessionStore.getState()
  const volumePath = firstVolumePath(input.tree)
  if (!volumePath) {
    const reply = '未找到卷，请先创建卷'
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  const callId = createCallId('tool')
  const res = await toolGateway.create({
    project_path: input.projectPath,
    kind: 'chapter',
    volume_path: volumePath,
    title: input.title,
    call_id: callId,
  })

  session.pushTrace(toTrace(input.turn, 'create', callId, res))

  if (!res.ok) {
    const reply = `create 失败: ${res.error?.fault_domain}/${res.error?.code}`
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  session.setActiveChapterPath(res.data?.path)
  const reply = `create 成功: ${res.data?.path}`
  session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
  return reply
}

type ReadTurnInput = {
  turn: number
  projectPath: string
  chapterPath: string
  view: 'snapshot' | 'json'
}

async function handleReadTurn(input: ReadTurnInput): Promise<string> {
  const session = useAgentSessionStore.getState()
  const callId = createCallId('tool')
  const res = await toolGateway.read({
    project_path: input.projectPath,
    kind: 'chapter',
    path: input.chapterPath,
    view: input.view,
    call_id: callId,
  })

  session.pushTrace(toTrace(input.turn, 'read', callId, res))

  if (!res.ok) {
    const reply = `read 失败: ${res.error?.fault_domain}/${res.error?.code}`
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  const reply = input.view === 'snapshot'
    ? `read snapshot 成功 (rev=${res.data?.revision})`
    : 'read json 成功'
  session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
  return reply
}

type EditTurnInput = {
  turn: number
  projectPath: string
  chapterPath: string
  dryRun: boolean
}

async function handleEditTurn(input: EditTurnInput): Promise<string> {
  const session = useAgentSessionStore.getState()
  const readCallId = createCallId('tool')
  const readHead = await toolGateway.read({
    project_path: input.projectPath,
    kind: 'chapter',
    path: input.chapterPath,
    view: 'snapshot',
    call_id: readCallId,
  })

  session.pushTrace(toTrace(input.turn, 'read', readCallId, readHead))

  if (!readHead.ok) {
    const reply = `read 失败: ${readHead.error?.fault_domain}/${readHead.error?.code}`
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  const snapshot = readHead.data?.snapshot as
    | { snapshot_id?: string; blocks?: Array<{ block_id?: string }> }
    | undefined
  const snapshotId = typeof snapshot?.snapshot_id === 'string' ? snapshot.snapshot_id : ''
  if (!snapshotId) {
    const reply = 'read 失败: snapshot 缺失'
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  const firstBlockId = Array.isArray(snapshot.blocks)
    ? snapshot.blocks.find((block) => typeof block.block_id === 'string' && block.block_id.trim().length > 0)?.block_id
    : undefined

  const ops = firstBlockId
    ? [{ op: 'replace_block' as const, block_id: firstBlockId, markdown: TEST_EDIT_MARKDOWN }]
    : [{ op: 'append_blocks' as const, blocks: [{ markdown: TEST_EDIT_MARKDOWN }] }]

  const callId = createCallId('tool')
  const res = await toolGateway.edit({
    project_path: input.projectPath,
    path: input.chapterPath,
    target: 'chapter_content',
    call_id: callId,
    base_revision: readHead.data?.revision ?? 0,
    snapshot_id: snapshotId,
    ops,
    dry_run: input.dryRun,
    actor: 'agent',
  })

  session.pushTrace(toTrace(input.turn, 'edit', callId, res))

  if (!res.ok) {
    const reply = `edit 失败: ${res.error?.fault_domain}/${res.error?.code}`
    session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
    return reply
  }

  const reply = input.dryRun
    ? `preview 成功: diagnostics=${res.data?.diagnostics.length ?? 0}`
    : `commit 成功: rev ${res.data?.revision_before} -> ${res.data?.revision_after}`
  session.pushMessage({ role: 'assistant', content: reply, ts: Date.now() })
  return reply
}

function toTrace(
  turn: number,
  toolName: 'create' | 'read' | 'edit',
  callId: string,
  result: ToolResult<unknown>,
) {
  if (result.ok) {
    return traceSuccess({
      turn,
      call_id: callId,
      tool_name: toolName,
      duration_ms: result.meta.duration_ms,
    })
  }

  return traceError({
    turn,
    call_id: callId,
    tool_name: toolName,
    duration_ms: result.meta.duration_ms,
    fault_domain: result.error?.fault_domain,
    error_code: result.error?.code,
  })
}

function firstVolumePath(tree: unknown[]): string | null {
  const nodes = Array.isArray(tree) ? tree : []
  for (const n of nodes) {
    const node = n as { kind?: string; path?: string }
    if (node.kind === 'dir' && node.path) return node.path
  }
  return null
}

function inferFirstChapterPath(tree: unknown[]): string | null {
  const nodes = Array.isArray(tree) ? tree : []
  for (const n of nodes) {
    const node = n as { kind?: string; children?: unknown[] }
    if (node.kind !== 'dir' || !Array.isArray(node.children)) continue
    for (const c of node.children) {
      const chapter = c as { kind?: string; path?: string }
      if (chapter.kind === 'chapter' && chapter.path) return chapter.path
    }
  }
  return null
}

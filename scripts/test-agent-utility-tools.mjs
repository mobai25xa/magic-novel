import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

function has(content, ...needles) {
  return needles.every((needle) => content.includes(needle))
}

async function main() {
  const root = resolve(import.meta.dirname, '..')

  const files = {
    agentChatIndex: await readFile(resolve(root, 'src/lib/agent-chat/index.ts'), 'utf-8'),
    askuserParser: await readFile(resolve(root, 'src/lib/agent-chat/askuser.ts'), 'utf-8'),
    todoParser: await readFile(resolve(root, 'src/lib/agent-chat/todo.ts'), 'utf-8'),
    sessionReplay: await readFile(resolve(root, 'src/lib/agent-chat/session/session-replay.ts'), 'utf-8'),
    sessionReducer: await readFile(resolve(root, 'src/lib/agent-chat/session/session-reducer.ts'), 'utf-8'),
    runtime: await readFile(resolve(root, 'src/lib/agent-chat/runtime.ts'), 'utf-8'),
    runtimeEvents: [
      await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/askuser.ts'), 'utf-8'),
      await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/handlers/tool.ts'), 'utf-8'),
      await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/tool-refresh.ts'), 'utf-8'),
    ].join('\n'),
    toolRefresh: await readFile(resolve(root, 'src/lib/agent-chat/runtime-backend-events/tool-refresh.ts'), 'utf-8'),
    toolStepUtils: await readFile(resolve(root, 'src/lib/agent-chat/tool-step-utils.ts'), 'utf-8'),
    store: await readFile(resolve(root, 'src/lib/agent-chat/store.ts'), 'utf-8'),
    timelineAskUser: await readFile(resolve(root, 'src/components/ai/timeline/TimelineBlockToolCall.tsx'), 'utf-8'),
    toolViewDispatcher: await readFile(resolve(root, 'src/components/ai/tool/ToolViewDispatcher.tsx'), 'utf-8'),
    toolIconMap: await readFile(resolve(root, 'src/components/ai/tool/tool-icon-map.ts'), 'utf-8'),
    toolGatewayTypes: await readFile(resolve(root, 'src/lib/tool-gateway/types.ts'), 'utf-8'),
    persistence: await readFile(resolve(root, 'src-tauri/src/agent_engine/persistence.rs'), 'utf-8'),
  }

  const checks = [
    {
      name: 'a0_agent_chat_surface_exports',
      pass:
        has(files.agentChatIndex,
          "export * from './store'",
          "export * from './runtime'",
          "export * from './runtime-backend-events'",
          "export * from './session'",
        ),
    },
    {
      name: 'a5_askuser_parse_and_name_rules',
      pass:
        has(files.askuserParser,
          'ASKUSER_MIN_QUESTIONS = 1',
          'ASKUSER_MAX_QUESTIONS = 4',
          'ASKUSER_MIN_OPTIONS = 2',
          'ASKUSER_MAX_OPTIONS = 4',
          "normalized === 'askuser'",
          "code: 'E_TOOL_SCHEMA_INVALID'",
          'question index must be continuous',
        )
        && !files.askuserParser.includes('mapLegacyAskUserQuestions'),
    },
    {
      name: 'a5_askuser_submit_cancel_and_resume',
      pass:
        has(files.store,
          'openAskUserRequest: (request) => set({ pendingAskUser: request })',
          'reduceMarkAskUserStepAnswered',
          'agentTurnResumeClient({',
          "kind: 'askuser'",
        ) &&
        has(files.runtime,
          'cancelCurrentChatTurn',
          'if (store.pendingAskUser) {',
          'store.cancelAskUserRequest(callId)',
          'agentTurnCancelClient({',
          'turn_id: turn',
        ) &&
        has(files.runtimeEvents,
          'store.openAskUserRequest({',
          'store.clearPendingAskUser()',
        ) &&
        has(files.timelineAskUser,
          'isAskUserToolName(input.step.toolName)',
        ),
    },
    {
      name: 'a5_askuser_persistence_and_replay',
      pass:
        has(files.persistence,
          '"call_id": envelope.payload.get("call_id")',
          '"tool_name": envelope.payload.get("tool_name")',
          '"questions": envelope.payload.get("questions")',
          '"questionnaire": envelope.payload.get("questionnaire")',
        ) &&
        has(files.sessionReplay,
          'function restorePendingAskUserRequest',
          'const pendingAskUser = restorePendingAskUserRequest({',
          'answeredAskUserCallIds',
          'state.pendingAskUser = undefined',
        ) &&
        has(files.sessionReducer,
          'pendingAskUser?: AgentPendingAskUserRequest',
          'pendingAskUser: replay.pendingAskUser',
        ),
    },
    {
      name: 'b5_todowrite_parse_and_replay',
      pass:
        has(files.todoParser,
          'MAX_TODO_ITEMS = 50',
          'MAX_TODO_TEXT_LENGTH = 500',
          'normalizeInProgressItems',
          'parseTodoWriteInputV2',
          'normalizeTodoStateFromToolResultPayload',
        ) &&
        has(files.runtimeEvents,
          'normalizeTodoStateFromToolResultPayload',
          'store.applyTodoState(todoState)',
        ) &&
        !files.runtimeEvents.includes('queueTodoStatePersistence') &&
        has(files.sessionReplay,
          'normalizeTodoStateFromToolResultPayload',
          'state.todoState = todoStateFromToolResult',
        )
        && !files.sessionReplay.includes('normalizeTodoStateFromEventPayload'),
    },
    {
      name: 'c5_active_skill_replay_state',
      pass:
        has(files.sessionReplay,
          "if (payload.kind !== 'skill_enabled')",
          'state.activeSkill = skill',
        ) &&
        has(files.sessionReducer,
          'activeSkill: replay.activeSkill',
        ),
    },
    {
      name: 'utility_tool_names_declared',
      pass:
        has(files.toolGatewayTypes,
          "| 'askuser'",
          "| 'skill'",
          "| 'todowrite'",
        ),
    },
    {
      name: 'writing_tool_names_include_delete_move',
      pass:
        has(files.toolGatewayTypes,
          "| 'delete'",
          "| 'move'",
          'export interface ToolDeleteInput',
          'export interface ToolMoveInput',
        ),
    },
    {
      name: 'tool_step_summary_handles_delete_move',
      pass:
        has(files.toolStepUtils,
          "if (toolName === 'delete')",
          "if (toolName === 'move')",
          "delete: ['kind', 'path', 'dry_run']",
          "move: ['chapter_path', 'target_volume_path', 'target_index', 'dry_run']",
        ),
    },
    {
      name: 'runtime_events_do_not_drop_delete_move',
      pass:
        has(files.toolRefresh,
          "toolName !== 'create'",
          "toolName !== 'edit'",
          "toolName !== 'delete'",
          "toolName !== 'move'",
          "toolName === 'move' && Boolean(chapterPath)",
          'extractToolPreviewRefs',
        ),
    },
    {
      name: 'tool_timeline_dispatcher_and_icons_cover_delete_move',
      pass:
        has(files.toolViewDispatcher,
          "case 'delete':",
          "case 'move':",
        ) &&
        has(files.toolIconMap,
          "delete: 'delete'",
          "move: 'move'",
        ),
    },
  ]

  const allPass = checks.every((check) => check.pass)
  console.log('[test-agent-utility-tools]', JSON.stringify({ all_pass: allPass, checks }))

  if (!allPass) {
    process.exit(1)
  }
}

main().catch((error) => {
  console.error('[test-agent-utility-tools] failed:', error.message)
  process.exit(1)
})

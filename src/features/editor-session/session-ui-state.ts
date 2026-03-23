import { mkdir, readTextFile, writeTextFile } from '@tauri-apps/plugin-fs'

import { openEditorTarget } from '@/features/editor-navigation/open-editor-target'
import { useAgentChatStore } from '@/state/agent'
import { useEditorStore } from '@/state/editor'
import { useProjectStore } from '@/state/project'
import { useEditorUiStore, type LeftPanelTab } from '@/stores/editor-ui-store'

export type SessionUiStateSchemaVersion = 1

export type SessionUiState = {
  schema_version: SessionUiStateSchemaVersion
  active_scope_refs: string[]
  editor_state: { opened_ref?: string } | null
  sidebar_state: {
    left_tab: LeftPanelTab
    selected_ref: string | null
    expanded_refs: string[]
    collapsed_refs: string[]
  }
}

type AgentSessionSettings = {
  schema_version: number
  session_id: string
  model?: string
  provider?: string
  token_budget?: number
  metadata?: Record<string, unknown>
}

function normalizeFsPath(path: string) {
  return path.replace(/\\/g, '/').replace(/\/+$/, '')
}

function isMissingFileError(error: unknown) {
  const text = String((error as { message?: unknown } | null)?.message ?? error ?? '')
  const lower = text.toLowerCase()
  return lower.includes('not found')
    || lower.includes('no such file')
    || lower.includes('os error 2')
}

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return null
  }
  return value as Record<string, unknown>
}

function asString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined
  }
  const trimmed = value.trim()
  return trimmed || undefined
}

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return []
  }
  return value
    .filter((item): item is string => typeof item === 'string')
    .map((item) => item.trim())
    .filter(Boolean)
}

function coerceLeftTab(value: unknown): LeftPanelTab | null {
  if (value === 'outline' || value === 'knowledge') {
    return value
  }
  return null
}

function sessionSettingsDir(projectPath: string) {
  return `${normalizeFsPath(projectPath)}/magic_novel/ai/sessions`
}

function sessionSettingsPath(projectPath: string, sessionId: string) {
  return `${sessionSettingsDir(projectPath)}/${sessionId}.settings.json`
}

function coerceSessionUiState(value: unknown): SessionUiState | null {
  const record = asRecord(value)
  if (!record) return null

  const schemaVersion = record.schema_version
  if (schemaVersion !== 1) return null

  const activeScopeRefs = asStringArray(record.active_scope_refs)

  const editorStateRecord = asRecord(record.editor_state)
  const editorStateOpenedRef = asString(editorStateRecord?.opened_ref)
  const editorStateKind = asString(editorStateRecord?.kind)
  const editorStateChapterPath = asString(editorStateRecord?.chapter_path)
  const editorStateAssetPath = asString(editorStateRecord?.asset_path)

  const editorState: SessionUiState['editor_state'] = editorStateOpenedRef
    ? { opened_ref: editorStateOpenedRef }
    : editorStateKind === 'chapter' && editorStateChapterPath
      ? { opened_ref: `chapter:${editorStateChapterPath}` }
      : editorStateKind === 'asset' && editorStateAssetPath
        ? { opened_ref: `asset:${editorStateAssetPath}` }
        : null

  const sidebarRecord = asRecord(record.sidebar_state)
  const leftTab = coerceLeftTab(sidebarRecord?.left_tab) ?? 'outline'
  const selectedRef = asString(sidebarRecord?.selected_ref) ?? asString(sidebarRecord?.selected_path) ?? null
  const expandedRefs = asStringArray(sidebarRecord?.expanded_refs)

  const legacyOutlineOpenPaths = asStringArray(sidebarRecord?.outline_open_paths)
  const legacyKnowledgeOpenPaths = asStringArray(sidebarRecord?.knowledge_open_paths)
  const legacyExpandedRefs = [...legacyOutlineOpenPaths, ...legacyKnowledgeOpenPaths]

  const collapsedRefs = asStringArray(sidebarRecord?.collapsed_refs)

  return {
    schema_version: 1,
    active_scope_refs: activeScopeRefs,
    editor_state: editorState,
    sidebar_state: {
      left_tab: leftTab,
      selected_ref: selectedRef,
      expanded_refs: expandedRefs.length > 0 ? expandedRefs : legacyExpandedRefs,
      collapsed_refs: collapsedRefs,
    },
  }
}

async function readAgentSessionSettingsFile(input: { projectPath: string; sessionId: string }): Promise<AgentSessionSettings | null> {
  const path = sessionSettingsPath(input.projectPath, input.sessionId)

  try {
    const raw = await readTextFile(path)
    if (!raw || !raw.trim()) {
      return null
    }

    const parsed = JSON.parse(raw) as unknown
    const record = asRecord(parsed)
    if (!record) {
      return null
    }

    const schemaVersion = record.schema_version
    const sessionId = asString(record.session_id)
    if (schemaVersion !== 1 || !sessionId) {
      return null
    }

    return {
      schema_version: 1,
      session_id: sessionId,
      model: asString(record.model),
      provider: asString(record.provider),
      token_budget: typeof record.token_budget === 'number' ? record.token_budget : undefined,
      metadata: asRecord(record.metadata) ?? undefined,
    }
  } catch (error) {
    if (isMissingFileError(error)) {
      return null
    }

    console.warn('[session-ui-state] Failed to read session settings:', error)
    return null
  }
}

async function writeAgentSessionSettingsFile(input: { projectPath: string; sessionId: string; settings: AgentSessionSettings }) {
  const dir = sessionSettingsDir(input.projectPath)
  const path = sessionSettingsPath(input.projectPath, input.sessionId)
  await mkdir(dir, { recursive: true })
  await writeTextFile(path, JSON.stringify(input.settings, null, 2))
}

export async function loadSessionUiStateFromDisk(input: { projectPath: string; sessionId: string }): Promise<SessionUiState | null> {
  const settings = await readAgentSessionSettingsFile(input)
  const uiState = settings?.metadata ? coerceSessionUiState(settings.metadata.ui_state) : null
  return uiState
}

export async function saveSessionUiStateToDisk(input: { projectPath: string; sessionId: string; uiState: SessionUiState }): Promise<void> {
  const existing = await readAgentSessionSettingsFile({ projectPath: input.projectPath, sessionId: input.sessionId })
  const metadata = { ...(existing?.metadata ?? {}) }
  metadata.ui_state = input.uiState

  await writeAgentSessionSettingsFile({
    projectPath: input.projectPath,
    sessionId: input.sessionId,
    settings: {
      schema_version: 1,
      session_id: input.sessionId,
      model: existing?.model,
      provider: existing?.provider,
      token_budget: existing?.token_budget,
      metadata,
    },
  })
}

export function buildSessionUiStateSnapshot(): SessionUiState {
  const agent = useAgentChatStore.getState()
  const editor = useEditorStore.getState()
  const project = useProjectStore.getState()
  const ui = useEditorUiStore.getState()

  const activeChapterPath = typeof agent.active_chapter_path === 'string' && agent.active_chapter_path.trim()
    ? agent.active_chapter_path.trim()
    : editor.currentDocKind === 'chapter' && editor.currentChapterPath
      ? editor.currentChapterPath
      : ''

  const activeScopeRefs = activeChapterPath ? [`chapter:${activeChapterPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')}`] : []

  const editorState: SessionUiState['editor_state'] =
    editor.currentDocKind === 'chapter' && editor.currentChapterPath
      ? { opened_ref: `chapter:${editor.currentChapterPath.replace(/\\/g, '/').replace(/^manuscripts\//, '')}` }
      : editor.currentDocKind === 'knowledge' && editor.currentAssetPath
        ? { opened_ref: `knowledge:${editor.currentAssetPath.replace(/\\/g, '/')}` }
      : editor.currentDocKind === 'asset' && editor.currentAssetPath
        ? { opened_ref: `asset:${editor.currentAssetPath.replace(/\\/g, '/').replace(/^assets\//, '')}` }
        : null

  const knownDirPaths = Object.keys(ui.sidebarTreeKnownDirPaths)
  const collapsedDirPaths = Object.keys(ui.sidebarTreeCollapsedDirPaths)
  const collapsedSet = new Set(collapsedDirPaths)
  const expandedDirPaths = knownDirPaths.filter((path) => !collapsedSet.has(path))

  return {
    schema_version: 1,
    active_scope_refs: activeScopeRefs,
    editor_state: editorState,
    sidebar_state: {
      left_tab: ui.leftPanelTab,
      selected_ref: project.selectedPath,
      expanded_refs: expandedDirPaths,
      collapsed_refs: collapsedDirPaths,
    },
  }
}

export async function applySessionUiState(state: SessionUiState): Promise<void> {
  const ui = useEditorUiStore.getState()
  ui.setLeftPanelTab(state.sidebar_state.left_tab)

  ui.setSidebarTreeCollapsedDirPaths(state.sidebar_state.collapsed_refs || [])

  const selectedRef = state.sidebar_state.selected_ref
  if (selectedRef) {
    useProjectStore.getState().setSelectedPath(selectedRef)
  }

  const openedRef = state.editor_state?.opened_ref
  if (openedRef) {
    await openEditorTarget(openedRef)
    return
  }

  const fallbackScope = state.active_scope_refs.at(0)
  if (fallbackScope) {
    await openEditorTarget(fallbackScope)
  }
}

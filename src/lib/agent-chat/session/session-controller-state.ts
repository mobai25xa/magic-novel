import {
  AGENT_SESSION_SCHEMA_VERSION,
  type AgentSessionMeta,
} from './session-types'

interface ActiveSession {
  projectPath: string
  sessionId: string
}

let activeSession: ActiveSession | null = null

function nowTs() {
  return Date.now()
}

export function hasActiveSession(input: { projectPath: string; sessionId: string }) {
  return activeSession?.projectPath === input.projectPath
    && activeSession?.sessionId === input.sessionId
}

export function setActiveSession(input: {
  projectPath: string
  sessionId: string
}) {
  activeSession = {
    projectPath: input.projectPath,
    sessionId: input.sessionId,
  }
}

export function getActiveSession() {
  if (!activeSession) {
    return null
  }

  return {
    projectPath: activeSession.projectPath,
    sessionId: activeSession.sessionId,
  }
}

export function clearSessionControllerState() {
  activeSession = null
}

export function buildEphemeralMeta(input: {
  projectPath: string
  title?: string
  activeChapterPath?: string
}): AgentSessionMeta | null {
  if (!activeSession || activeSession.projectPath !== input.projectPath) {
    return null
  }

  return {
    schema_version: AGENT_SESSION_SCHEMA_VERSION,
    session_id: activeSession.sessionId,
    created_at: nowTs(),
    updated_at: nowTs(),
    title: input.title,
    active_chapter_path: input.activeChapterPath,
    compaction_count: 0,
  }
}

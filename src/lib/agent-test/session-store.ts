import { create } from 'zustand'

import type { ToolTrace } from './trace'

export interface AgentMessage {
  role: 'user' | 'assistant' | 'system'
  content: string
  ts: number
}

export interface AgentSessionState {
  session_id: string
  turn: number
  active_chapter_path?: string
  messages: AgentMessage[]
  traces: ToolTrace[]

  setActiveChapterPath: (path?: string) => void
  pushMessage: (message: AgentMessage) => void
  pushTrace: (trace: ToolTrace) => void
  nextTurn: () => number
  reset: () => void
}

function newSessionId() {
  return `agent_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`
}

const initialState = () => ({
  session_id: newSessionId(),
  turn: 0,
  active_chapter_path: undefined as string | undefined,
  messages: [] as AgentMessage[],
  traces: [] as ToolTrace[],
})

export const useAgentSessionStore = create<AgentSessionState>((set, get) => ({
  ...initialState(),

  setActiveChapterPath: (path) => set({ active_chapter_path: path }),

  pushMessage: (message) => set((state) => ({ messages: [...state.messages, message] })),

  pushTrace: (trace) => set((state) => ({ traces: [...state.traces, trace] })),

  nextTurn: () => {
    const next = get().turn + 1
    set({ turn: next })
    return next
  },

  reset: () => set(initialState()),
}))

import {
  useCallback,
  useEffect,
  useRef,
  type MutableRefObject,
} from 'react'
import type { Editor } from '@tiptap/react'

import {
  endWritingSessionFeature,
  startWritingSessionFeature,
  updateWritingSessionFeature,
} from '@/features/content-editing'

const SESSION_UPDATE_INTERVAL = 30000
const IDLE_THRESHOLD = 60000

type SessionInput = {
  editor: Editor | null
  projectPath: string | null
  activeDocPath: string | null
  projectsRootDir: string | null
}

type SessionRefs = {
  sessionIdRef: MutableRefObject<string | null>
  lastActivityTimeRef: MutableRefObject<number>
  activeDurationRef: MutableRefObject<number>
  idleDurationRef: MutableRefObject<number>
  initialWordCountRef: MutableRefObject<number>
}

function useSessionRefs(): SessionRefs {
  const sessionIdRef = useRef<string | null>(null)
  const lastActivityTimeRef = useRef<number>(0)
  const activeDurationRef = useRef<number>(0)
  const idleDurationRef = useRef<number>(0)
  const initialWordCountRef = useRef<number>(0)

  return {
    sessionIdRef,
    lastActivityTimeRef,
    activeDurationRef,
    idleDurationRef,
    initialWordCountRef,
  }
}

function useWordCounter(editor: Editor | null) {
  return useCallback(() => {
    if (!editor) return 0
    return editor.getText().replace(/\s/g, '').length
  }, [editor])
}

function useEndSession(
  projectsRootDir: string | null,
  refs: SessionRefs,
  getWordCount: () => number,
) {
  const { sessionIdRef, initialWordCountRef } = refs

  return useCallback(async () => {
    if (!sessionIdRef.current) return

    try {
      const finalWordCount = getWordCount()
      if (finalWordCount === 0 && initialWordCountRef.current > 0) return
      await endWritingSessionFeature(finalWordCount, projectsRootDir || undefined)
      sessionIdRef.current = null
    } catch (error) {
      console.error('Failed to end writing session:', error)
    }
  }, [getWordCount, initialWordCountRef, projectsRootDir, sessionIdRef])
}

function useStartSession(
  input: SessionInput,
  refs: SessionRefs,
  getWordCount: () => number,
  endSession: () => Promise<void>,
) {
  const {
    sessionIdRef,
    initialWordCountRef,
    lastActivityTimeRef,
    activeDurationRef,
    idleDurationRef,
  } = refs

  return useCallback(async () => {
    if (!input.projectPath) return

    if (sessionIdRef.current) {
      await endSession()
    }

    try {
      const wordCount = getWordCount()
      initialWordCountRef.current = wordCount
      lastActivityTimeRef.current = Date.now()
      activeDurationRef.current = 0
      idleDurationRef.current = 0

      sessionIdRef.current = await startWritingSessionFeature(
        input.projectPath,
        input.activeDocPath || null,
        wordCount,
        input.projectsRootDir || undefined,
      )
    } catch (error) {
      console.error('Failed to start writing session:', error)
    }
  }, [
    activeDurationRef,
    endSession,
    getWordCount,
    idleDurationRef,
    initialWordCountRef,
    input.activeDocPath,
    input.projectPath,
    input.projectsRootDir,
    lastActivityTimeRef,
    sessionIdRef,
  ])
}

function useUpdateSession(
  projectsRootDir: string | null,
  refs: SessionRefs,
  getWordCount: () => number,
) {
  const {
    sessionIdRef,
    lastActivityTimeRef,
    activeDurationRef,
    idleDurationRef,
  } = refs

  return useCallback(async () => {
    if (!sessionIdRef.current) return

    try {
      const timeSinceLastActivity = Date.now() - lastActivityTimeRef.current
      if (timeSinceLastActivity < IDLE_THRESHOLD) {
        activeDurationRef.current += SESSION_UPDATE_INTERVAL / 1000
      } else {
        idleDurationRef.current += SESSION_UPDATE_INTERVAL / 1000
      }

      await updateWritingSessionFeature(
        getWordCount(),
        Math.round(activeDurationRef.current),
        Math.round(idleDurationRef.current),
        projectsRootDir || undefined,
      )
    } catch (error) {
      console.error('Failed to update writing session:', error)
    }
  }, [activeDurationRef, getWordCount, idleDurationRef, lastActivityTimeRef, projectsRootDir, sessionIdRef])
}

function useRecordActivity(refs: SessionRefs) {
  const { lastActivityTimeRef } = refs

  return useCallback(() => {
    lastActivityTimeRef.current = Date.now()
  }, [lastActivityTimeRef])
}

function useSessionStartEffect(
  editor: Editor | null,
  projectPath: string | null,
  activeDocPath: string | null,
  startSession: () => Promise<void>,
) {
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null
    let isActive = true

    if (editor && projectPath && activeDocPath) {
      timer = setTimeout(async () => {
        if (isActive) await startSession()
      }, 500)
    }

    return () => {
      isActive = false
      if (timer) clearTimeout(timer)
    }
  }, [activeDocPath, editor, projectPath, startSession])
}

function useSessionUnmountEffect(
  sessionIdRef: MutableRefObject<string | null>,
  endSession: () => Promise<void>,
) {
  useEffect(() => {
    const sessionIdRefCurrent = sessionIdRef.current
    return () => {
      if (sessionIdRefCurrent) {
        void endSession()
      }
    }
  }, [endSession, sessionIdRef])
}

function useSessionIntervalEffect(
  sessionIdRef: MutableRefObject<string | null>,
  updateSession: () => Promise<void>,
) {
  useEffect(() => {
    if (!sessionIdRef.current) return
    const interval = setInterval(updateSession, SESSION_UPDATE_INTERVAL)
    return () => clearInterval(interval)
  }, [sessionIdRef, updateSession])
}

function useEditorActivityEffect(editor: Editor | null, recordActivity: () => void) {
  useEffect(() => {
    if (!editor) return

    const handleUpdate = () => {
      recordActivity()
    }

    editor.on('update', handleUpdate)
    return () => {
      editor.off('update', handleUpdate)
    }
  }, [editor, recordActivity])
}

export function useAutoSaveSession(input: SessionInput) {
  const refs = useSessionRefs()
  const getWordCount = useWordCounter(input.editor)
  const endSession = useEndSession(input.projectsRootDir, refs, getWordCount)
  const startSession = useStartSession(input, refs, getWordCount, endSession)
  const updateSession = useUpdateSession(input.projectsRootDir, refs, getWordCount)
  const recordActivity = useRecordActivity(refs)

  useSessionStartEffect(input.editor, input.projectPath, input.activeDocPath, startSession)
  useSessionUnmountEffect(refs.sessionIdRef, endSession)
  useSessionIntervalEffect(refs.sessionIdRef, updateSession)
  useEditorActivityEffect(input.editor, recordActivity)
}

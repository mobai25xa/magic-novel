import { useCallback, useEffect, useMemo, useRef, useState, type RefObject } from 'react'

const NEAR_BOTTOM_THRESHOLD = 80

export type ChatScrollState = {
  autoScrollLocked: boolean
  unseenCount: number
  lastScrollTop: number
}

export function createInitialChatScrollState(): ChatScrollState {
  return {
    autoScrollLocked: false,
    unseenCount: 0,
    lastScrollTop: 0,
  }
}

export function isNearBottom(element: HTMLElement) {
  return element.scrollHeight - (element.scrollTop + element.clientHeight) <= NEAR_BOTTOM_THRESHOLD
}

export function updateScrollLockState(
  previous: ChatScrollState,
  container: HTMLElement,
): ChatScrollState {
  const nearBottom = isNearBottom(container)
  const scrolledUp = container.scrollTop < previous.lastScrollTop
  const nextScrollTop = container.scrollTop

  if (nearBottom) {
    return previous.autoScrollLocked
      ? { autoScrollLocked: false, unseenCount: 0, lastScrollTop: nextScrollTop }
      : { ...previous, lastScrollTop: nextScrollTop }
  }

  // Only lock when user actively scrolls up
  if (!previous.autoScrollLocked && scrolledUp) {
    return { autoScrollLocked: true, unseenCount: 0, lastScrollTop: nextScrollTop }
  }

  return { ...previous, lastScrollTop: nextScrollTop }
}

/**
 * Create a RAF-throttled scroll handler to prevent excessive re-renders.
 */
export function createScrollHandler(
  setScrollState: React.Dispatch<React.SetStateAction<ChatScrollState>>,
): (event: React.UIEvent<HTMLDivElement>) => void {
  let rafId: number | null = null

  return (event: React.UIEvent<HTMLDivElement>) => {
    // Capture target before RAF callback (React synthetic event is pooled)
    const target = event.currentTarget
    if (rafId !== null) return

    rafId = requestAnimationFrame(() => {
      setScrollState((prev) => updateScrollLockState(prev, target))
      rafId = null
    })
  }
}

export function jumpToLatest(containerRef: RefObject<HTMLElement | null>): ChatScrollState {
  const el = containerRef.current
  if (el) {
    el.scrollTo({
      top: el.scrollHeight,
      behavior: 'smooth',
    })
  }
  return {
    autoScrollLocked: false,
    unseenCount: 0,
    lastScrollTop: el?.scrollHeight ?? 0,
  }
}

type UseChatTranscriptScrollInput = {
  contentSignature: string
  itemCount: number
  streaming: boolean
  sessionKey?: string | null
}

export function useChatTranscriptScroll(input: UseChatTranscriptScrollInput) {
  const sessionKey = input.sessionKey ?? null
  const scrollRef = useRef<HTMLDivElement | null>(null)
  const [sessionScrollState, setSessionScrollState] = useState(() => ({
    sessionKey,
    value: createInitialChatScrollState(),
  }))
  const autoScrollRafRef = useRef<number | null>(null)
  const pinBottomRafRef = useRef<number | null>(null)
  const prevItemCountRef = useRef<number>(input.itemCount)

  const cancelScrollRaf = useCallback((rafRef: React.MutableRefObject<number | null>) => {
    if (rafRef.current !== null) {
      cancelAnimationFrame(rafRef.current)
      rafRef.current = null
    }
  }, [])

  const scheduleScrollToBottom = useCallback((rafRef: React.MutableRefObject<number | null>) => {
    cancelScrollRaf(rafRef)
    rafRef.current = requestAnimationFrame(() => {
      const element = scrollRef.current
      if (element) {
        element.scrollTo({ top: element.scrollHeight, behavior: 'auto' })
      }
      rafRef.current = null
    })
  }, [cancelScrollRaf])

  const setActiveScrollState = useCallback((next: React.SetStateAction<ChatScrollState>) => {
    setSessionScrollState((prev) => {
      const current = prev.sessionKey === sessionKey
        ? prev.value
        : createInitialChatScrollState()
      const value = typeof next === 'function'
        ? next(current)
        : next

      return { sessionKey, value }
    })
  }, [sessionKey])

  const scrollState = useMemo(() => (
    sessionScrollState.sessionKey === sessionKey
      ? sessionScrollState.value
      : createInitialChatScrollState()
  ), [sessionKey, sessionScrollState])
  const handleScroll = useMemo(() => createScrollHandler(setActiveScrollState), [setActiveScrollState])

  const jumpToLatestAction = useCallback(() => {
    setActiveScrollState(jumpToLatest(scrollRef))
  }, [setActiveScrollState])

  useEffect(() => {
    return () => {
      cancelScrollRaf(autoScrollRafRef)
      cancelScrollRaf(pinBottomRafRef)
    }
  }, [cancelScrollRaf])

  useEffect(() => {
    cancelScrollRaf(autoScrollRafRef)
    cancelScrollRaf(pinBottomRafRef)
    prevItemCountRef.current = input.itemCount
  }, [cancelScrollRaf, input.itemCount, sessionKey])

  useEffect(() => {
    if (!input.contentSignature) {
      return
    }

    setActiveScrollState((previous) => {
      const element = scrollRef.current
      if (!element) {
        return previous
      }

      if (!previous.autoScrollLocked) {
        scheduleScrollToBottom(autoScrollRafRef)
        return { ...previous, unseenCount: 0 }
      }

      return {
        ...previous,
        unseenCount: previous.unseenCount + 1,
      }
    })
  }, [input.contentSignature, scheduleScrollToBottom, setActiveScrollState])

  useEffect(() => {
    if (!scrollRef.current) {
      return
    }
    setActiveScrollState((previous) => updateScrollLockState(previous, scrollRef.current!))
  }, [input.streaming, setActiveScrollState])

  useEffect(() => {
    if (!input.streaming) {
      cancelScrollRaf(pinBottomRafRef)
      prevItemCountRef.current = input.itemCount
      return
    }

    const element = scrollRef.current
    if (!element || scrollState.autoScrollLocked) {
      prevItemCountRef.current = input.itemCount
      return
    }

    const nearBottom = element.scrollHeight - (element.scrollTop + element.clientHeight) <= 120
    const newItemAppended = input.itemCount > prevItemCountRef.current

    if (!nearBottom && !newItemAppended) {
      prevItemCountRef.current = input.itemCount
      return
    }

    scheduleScrollToBottom(pinBottomRafRef)
    prevItemCountRef.current = input.itemCount

    return () => {
      cancelScrollRaf(pinBottomRafRef)
    }
  }, [
    cancelScrollRaf,
    input.itemCount,
    input.streaming,
    scheduleScrollToBottom,
    scrollState.autoScrollLocked,
  ])

  return {
    scrollRef,
    scrollState,
    handleScroll,
    jumpToLatest: jumpToLatestAction,
  }
}

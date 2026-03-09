import type { RefObject } from 'react'

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

// Module-level pending flag for auto-scroll coalescing
let pendingAutoScroll = false

function scheduleAutoScroll(el: HTMLElement) {
  if (pendingAutoScroll) return
  pendingAutoScroll = true

  requestAnimationFrame(() => {
    el.scrollTo({ top: el.scrollHeight, behavior: 'auto' })
    pendingAutoScroll = false
  })
}

export function handleIncomingContent(
  previous: ChatScrollState,
  containerRef: RefObject<HTMLElement | null>,
  hasIncomingContent: boolean,
): ChatScrollState {
  if (!hasIncomingContent) {
    return previous
  }

  const el = containerRef.current
  if (!el) {
    return previous
  }

  if (!previous.autoScrollLocked) {
    scheduleAutoScroll(el)
    return { ...previous, unseenCount: 0 }
  }

  return {
    ...previous,
    unseenCount: previous.unseenCount + 1,
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

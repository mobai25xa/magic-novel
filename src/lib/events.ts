// Simple event emitter for cross-component communication

type EventCallback = (...args: unknown[]) => void

class EventEmitter {
  private events: Map<string, Set<EventCallback>> = new Map()

  on(event: string, callback: EventCallback) {
    if (!this.events.has(event)) {
      this.events.set(event, new Set())
    }
    this.events.get(event)!.add(callback)
  }

  off(event: string, callback: EventCallback) {
    const callbacks = this.events.get(event)
    if (callbacks) {
      callbacks.delete(callback)
    }
  }

  emit(event: string, ...args: unknown[]) {
    const callbacks = this.events.get(event)
    if (callbacks) {
      callbacks.forEach(callback => callback(...args))
    }
  }
}

// Global event emitter instance
export const eventBus = new EventEmitter()

// Event types
export const EVENTS = {
  CHAPTER_SAVED: 'chapter:saved',
  STATS_REFRESH_NEEDED: 'stats:refresh',
  FIND_REPLACE_OPEN: 'find-replace:open',
  FIND_REPLACE_CLOSE: 'find-replace:close',
  FULLSCREEN_TOGGLE: 'fullscreen:toggle',
  EDITOR_READY: 'editor:ready',
  EDITOR_DESTROYED: 'editor:destroyed',
  RECYCLE_SEARCH_CHANGED: 'recycle:search-changed',
  RECYCLE_EMPTY_ALL_REQUESTED: 'recycle:empty-all-requested',
  RECYCLE_REFRESH_REQUESTED: 'recycle:refresh-requested',
} as const

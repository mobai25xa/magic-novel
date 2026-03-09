export type HeightCache = {
  get(key: string): number | undefined
  set(key: string, height: number): void
  remove(key: string): void
  clear(): void
  totalHeight(): number
  size(): number
}

export function createHeightCache(): HeightCache {
  const store = new Map<string, number>()

  return {
    get(key) {
      return store.get(key)
    },
    set(key, height) {
      store.set(key, height)
    },
    remove(key) {
      store.delete(key)
    },
    clear() {
      store.clear()
    },
    totalHeight() {
      let total = 0
      for (const h of store.values()) {
        total += h
      }
      return total
    },
    size() {
      return store.size
    },
  }
}

import { useEffect, useState, type RefObject } from 'react'

type UseVisibleOptions = {
  rootMargin?: string
  once?: boolean
}

export function useVisible(
  ref: RefObject<HTMLElement | null>,
  options?: UseVisibleOptions,
): boolean {
  const [visible, setVisible] = useState(false)
  const rootMargin = options?.rootMargin ?? '100px'
  const once = options?.once ?? false

  useEffect(() => {
    const el = ref.current
    if (!el) return

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true)
          if (once) observer.disconnect()
        } else if (!once) {
          setVisible(false)
        }
      },
      { rootMargin },
    )

    observer.observe(el)
    return () => observer.disconnect()
  }, [ref, rootMargin, once])

  return visible
}

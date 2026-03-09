/**
 * @author Beta
 * @date 2026-02-11
 * @description 窗口关闭保护 — 拦截关闭事件，检查未保存修改
 */
import { useEffect, useRef } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEditorStore } from '@/stores/editor-store'

export function useCloseProtection() {
  const bypassRef = useRef(false)

  useEffect(() => {
    const appWindow = getCurrentWindow()

    const unlistenPromise = appWindow.onCloseRequested(async (event) => {
      // 当我们在 handler 内部触发二次 close 时，避免再次拦截导致无法退出
      if (bypassRef.current) return

      const { isDirty } = useEditorStore.getState()

      if (!isDirty) {
        // 没有未保存修改，让默认关闭行为执行
        return
      }

      // 阻止默认关闭
      event.preventDefault()

      // 尝试保存（失败也不应卡死关闭按钮）
      type ManualSaveFn = () => void | Promise<void>
      const manualSave = (window as unknown as { __manualSave?: ManualSaveFn }).__manualSave
      if (typeof manualSave === 'function') {
        try {
          await manualSave()
        } catch (error) {
          console.error('Failed to save before close:', error)
        }
      }

      // 保存完成后关闭（触发二次 close 时不再拦截，避免递归导致无法退出）
      bypassRef.current = true
      await appWindow.close()
    })

    return () => {
      unlistenPromise.then((fn) => fn())
    }
  }, [])
}

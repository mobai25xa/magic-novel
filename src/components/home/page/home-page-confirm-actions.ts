import type { HomePendingAction } from './home-page-types'

export function createConfirmPendingAction(input: {
  getPendingAction: () => HomePendingAction | null
  getIsMutating: () => boolean
  setIsMutating: (value: boolean) => void
  setConfirmDialog: (value: { open: boolean; title: string; description: string } | null) => void
  setPendingAction: (value: HomePendingAction | null) => void
  onMoveToRecycle: (path: string) => Promise<void>
  onPermanentDelete: (id: string) => Promise<void>
}) {
  return async () => {
    const pendingAction = input.getPendingAction()
    if (!pendingAction || input.getIsMutating()) return

    input.setIsMutating(true)
    try {
      if (pendingAction.type === 'move_to_recycle') {
        await input.onMoveToRecycle(pendingAction.path)
      } else {
        await input.onPermanentDelete(pendingAction.id)
      }

      input.setConfirmDialog(null)
      input.setPendingAction(null)
    } finally {
      input.setIsMutating(false)
    }
  }
}

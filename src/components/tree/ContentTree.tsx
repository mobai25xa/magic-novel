import { ConfirmDialog } from '@/components/common/ConfirmDialog'
import { useTranslation } from '@/hooks/use-translation'

import { useContentTreeController } from './content-tree-controller'
import { ContentTreeNode } from './node/content-tree-node'

type ContentTreeProps = {
  onChapterSelect: (chapterPath: string, chapterId: string, title?: string) => void
  onCreateChapterInVolume?: (volumePath: string) => void
  onAssetSelect?: (relativePath: string) => void
  mode?: 'all' | 'knowledge' | 'manuscript'
  hideKnowledgeRoot?: boolean
  variant?: 'tree' | 'outline'
}

export function ContentTree({
  onChapterSelect,
  onCreateChapterInVolume,
  onAssetSelect,
  mode = 'all',
  hideKnowledgeRoot = false,
  variant = 'tree',
}: ContentTreeProps) {
  const controller = useContentTreeController({ onChapterSelect, onAssetSelect, mode, hideKnowledgeRoot })
  const { translations } = useTranslation()

  if (controller.sortedTree.length === 0) {
    return null
  }

  return (
    <div className="py-2">
      {!controller.dragEnabled ? (
        <div className="px-3 pb-2 text-xs text-muted-foreground">{translations.tree.sortDisabledHint}</div>
      ) : null}

      {controller.sortedTree.map((node) => (
        <ContentTreeNode
          key={node.path}
          node={node}
          level={0}
          onSelect={controller.handleSelect}
          selectedPath={controller.selectedPath}
          onDelete={controller.handleDelete}
          onRename={controller.handleRename}
          onCreateChapter={onCreateChapterInVolume}
          onMoveChapter={controller.dragEnabled ? controller.handleMoveChapter : undefined}
          dragState={controller.dragState}
          setDragState={controller.setDragState}
          getSiblingIndex={controller.getSiblingIndex}
          variant={variant}
        />
      ))}

      {controller.confirmDialog ? (
        <ConfirmDialog
          open={controller.confirmDialog.open}
          title={controller.confirmDialog.title}
          description={controller.confirmDialog.description}
          danger={true}
          onConfirm={controller.confirmDialog.onConfirm}
          onCancel={() => {
            if (!controller.isDeleting) {
              controller.clearConfirmDialog()
            }
          }}
        />
      ) : null}
    </div>
  )
}

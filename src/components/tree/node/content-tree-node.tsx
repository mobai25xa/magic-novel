import { useToast } from '@/magic-ui/components'
import { useEditorStore } from '@/stores/editor-store'
import { useProjectStore } from '@/stores/project-store'
import { useTranslation } from '@/hooks/use-translation'

import type { TreeNodeProps } from '../content-tree-types'

import { createTreeNodeActions } from './content-tree-node-actions'
import { useContentTreeNodeController } from './content-tree-node-controller'
import { ContentTreeNodeView } from './content-tree-node-view'

export function ContentTreeNode(props: TreeNodeProps) {
  const { projectPath, setTree } = useProjectStore()
  const { currentAssetPath, setCurrentAsset } = useEditorStore()
  const { addToast } = useToast()
  const { translations } = useTranslation()

  const controller = useContentTreeNodeController(props)

  const actions = createTreeNodeActions({
    node: props.node,
    onSelect: props.onSelect,
    onDelete: props.onDelete,
    onRename: props.onRename,
    projectPath,
    setTree,
    addToast,
    currentAssetPath,
    setCurrentAsset,
    labels: translations.tree,
  })

  return <ContentTreeNodeView props={props} controller={controller} actions={actions} />
}

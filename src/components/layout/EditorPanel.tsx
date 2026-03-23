import { NovelEditor } from '@/components/editor/NovelEditor'
import { PendingExternalChapterRefreshBanner } from '@/components/editor/PendingExternalChapterRefreshBanner'
import { ProjectBootstrapStatusBanner } from '@/components/editor/ProjectBootstrapStatusBanner'
import { StatusBar } from '@/components/editor/StatusBar'
import { useEditorStore } from '@/stores/editor-store'
import { useLayoutStore } from '@/stores/layout-store'
import { useTranslation } from '@/hooks/use-translation'

export function EditorPanel() {
  const { currentDocKind, currentChapterId, currentAssetPath, content, setContent, editor } = useEditorStore()
  const { isFullscreen } = useLayoutStore()
  const { translations } = useTranslation()
  const lt = translations.layout

  const isAssetDoc = currentDocKind === 'asset' && Boolean(currentAssetPath)
  const isKnowledgeDoc = currentDocKind === 'knowledge' && Boolean(currentAssetPath)
  const isChapterDoc = currentDocKind === 'chapter' && Boolean(currentChapterId)

  if (!isAssetDoc && !isKnowledgeDoc && !isChapterDoc) {
    return (
      <div className="panel-editor flex items-center justify-center">
        <div className="text-center">
          <p className="text-lg mb-2" style={{ color: "var(--text-secondary-foreground)" }}>{lt.selectChapterToEdit}</p>
          <p className="text-sm" style={{ color: "var(--text-muted-foreground)" }}>{lt.orCreateNew}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="panel-editor">
      <ProjectBootstrapStatusBanner />
      <PendingExternalChapterRefreshBanner />
      <div className="flex-1 min-h-0 overflow-hidden">
        <NovelEditor
          initialContent={content as Parameters<typeof NovelEditor>[0]['initialContent']}
          onContentChange={setContent}
        />
      </div>
      {!isFullscreen && (isChapterDoc || isAssetDoc || isKnowledgeDoc) ? <StatusBar editor={editor} /> : null}
    </div>
  )
}

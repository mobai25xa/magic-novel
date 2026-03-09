import type { Editor } from '@tiptap/react'
import {
  Bold,
  Italic,
  Strikethrough,
  Highlighter,
  Heading1,
  Heading2,
  Heading3,
  Undo,
  Redo,
  Quote,
  Search,
  Minus,
  Maximize,
  Minimize,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/magic-ui/components'
import { useLayoutStore } from '@/stores/layout-store'
import { useTranslation } from '@/hooks/use-translation'
import { buildEditorToolbarActions } from './editor-toolbar-actions'

interface EditorToolbarProps {
  editor: Editor | null
  onToggleFindReplace?: () => void
}

interface ToolbarButtonProps {
  onClick: () => void
  isActive?: boolean
  disabled?: boolean
  title: string
  children: React.ReactNode
}

function ToolbarButton({ onClick, isActive, disabled, title, children }: ToolbarButtonProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          onClick={onClick}
          disabled={disabled}
          className={cn(
            'toolbar-btn',
            isActive && 'toolbar-btn-active'
          )}
        >
          {children}
        </button>
      </TooltipTrigger>
      <TooltipContent variant="success">
        {title}
      </TooltipContent>
    </Tooltip>
  )
}

export function EditorToolbar({ editor, onToggleFindReplace }: EditorToolbarProps) {
  const { isFullscreen, toggleFullscreen } = useLayoutStore()
  const { translations } = useTranslation()

  if (!editor) {
    return null
  }

  const iconByKey: Record<string, React.ReactNode> = {
    undo: <Undo className="h-4 w-4" />,
    redo: <Redo className="h-4 w-4" />,
    'heading-1': <Heading1 className="h-4 w-4" />,
    'heading-2': <Heading2 className="h-4 w-4" />,
    'heading-3': <Heading3 className="h-4 w-4" />,
    blockquote: <Quote className="h-4 w-4" />,
    'horizontal-rule': <Minus className="h-4 w-4" />,
    bold: <Bold className="h-4 w-4" />,
    italic: <Italic className="h-4 w-4" />,
    strike: <Strikethrough className="h-4 w-4" />,
    highlight: <Highlighter className="h-4 w-4" />,
    'find-replace': <Search className="h-4 w-4" />,
    fullscreen: isFullscreen ? <Minimize className="h-4 w-4" /> : <Maximize className="h-4 w-4" />,
  }

  const { actions, sectionOrder } = buildEditorToolbarActions({
    editor,
    isFullscreen,
    toggleFullscreen,
    onToggleFindReplace,
    toolbar: translations.toolbar,
  })

  return (
    <TooltipProvider>
      <div className="editor-toolbar">
        {sectionOrder.map((section, sectionIndex) => {
          const sectionActions = actions.filter((action) => action.section === section)
          if (sectionActions.length === 0) {
            return null
          }

          return (
            <div key={section} className="flex items-center gap-1">
              {sectionIndex > 0 ? <div className="editor-toolbar-sep" /> : null}
              {sectionActions.map((action) => (
                <ToolbarButton
                  key={action.key}
                  onClick={action.onClick}
                  disabled={action.disabled}
                  isActive={action.isActive}
                  title={action.title}
                >
                  {iconByKey[action.key]}
                </ToolbarButton>
              ))}
            </div>
          )
        })}
      </div>
    </TooltipProvider>
  )
}

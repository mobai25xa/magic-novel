# 前端编辑器开发文档

> 本文档详细描述 Tiptap 编辑器的配置和实现细节。

---

## 1. 编辑器架构

```
src/components/editor/
├── NovelEditor.tsx           # 主编辑器组件
├── EditorToolbar.tsx         # 工具栏组件
├── extensions/
│   └── unique-id.ts          # Block ID 扩展
└── index.ts                  # 导出
```

---

## 2. UniqueIdExtension 实现

### 2.1 完整代码 - extensions/unique-id.ts

```typescript
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { v4 as uuidv4 } from 'uuid';

const BLOCK_TYPES = ['paragraph', 'heading', 'blockquote'];

export const UniqueIdExtension = Extension.create({
  name: 'uniqueId',

  addGlobalAttributes() {
    return BLOCK_TYPES.map((type) => ({
      types: [type],
      attributes: {
        id: {
          default: null,
          parseHTML: () => uuidv4(),
          renderHTML: (attributes) => {
            if (!attributes.id) {
              return {};
            }
            return { 'data-id': attributes.id };
          },
        },
      },
    }));
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('uniqueId'),
        appendTransaction: (transactions, oldState, newState) => {
          const docChanged = transactions.some((tr) => tr.docChanged);
          if (!docChanged) {
            return null;
          }

          const tr = newState.tr;
          let modified = false;

          newState.doc.descendants((node, pos) => {
            if (BLOCK_TYPES.includes(node.type.name) && !node.attrs.id) {
              tr.setNodeMarkup(pos, undefined, {
                ...node.attrs,
                id: uuidv4(),
              });
              modified = true;
            }
          });

          return modified ? tr : null;
        },
      }),
    ];
  },
});
```

---

## 3. 编辑器工具栏

### 3.1 完整代码 - EditorToolbar.tsx

```tsx
import { Editor } from '@tiptap/react';
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
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

interface EditorToolbarProps {
  editor: Editor | null;
}

export function EditorToolbar({ editor }: EditorToolbarProps) {
  if (!editor) {
    return null;
  }

  const ToolbarButton = ({
    onClick,
    isActive,
    disabled,
    tooltip,
    children,
  }: {
    onClick: () => void;
    isActive?: boolean;
    disabled?: boolean;
    tooltip: string;
    children: React.ReactNode;
  }) => (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant={isActive ? 'secondary' : 'ghost'}
          size="sm"
          onClick={onClick}
          disabled={disabled}
          className="h-8 w-8 p-0"
        >
          {children}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        <p>{tooltip}</p>
      </TooltipContent>
    </Tooltip>
  );

  return (
    <div className="flex items-center gap-1 border-b px-2 py-1">
      <ToolbarButton
        onClick={() => editor.chain().focus().undo().run()}
        disabled={!editor.can().undo()}
        tooltip="撤销 (Ctrl+Z)"
      >
        <Undo className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => editor.chain().focus().redo().run()}
        disabled={!editor.can().redo()}
        tooltip="重做 (Ctrl+Y)"
      >
        <Redo className="h-4 w-4" />
      </ToolbarButton>

      <Separator orientation="vertical" className="mx-1 h-6" />

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="sm" className="h-8 gap-1 px-2">
            <Heading1 className="h-4 w-4" />
            <span className="text-xs">标题</span>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          <DropdownMenuItem
            onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
          >
            <Heading1 className="mr-2 h-4 w-4" />
            标题 1
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
          >
            <Heading2 className="mr-2 h-4 w-4" />
            标题 2
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={() => editor.chain().focus().toggleHeading({ level: 3 }).run()}
          >
            <Heading3 className="mr-2 h-4 w-4" />
            标题 3
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
        isActive={editor.isActive('blockquote')}
        tooltip="引用"
      >
        <Quote className="h-4 w-4" />
      </ToolbarButton>

      <Separator orientation="vertical" className="mx-1 h-6" />

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleBold().run()}
        isActive={editor.isActive('bold')}
        tooltip="粗体 (Ctrl+B)"
      >
        <Bold className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleItalic().run()}
        isActive={editor.isActive('italic')}
        tooltip="斜体 (Ctrl+I)"
      >
        <Italic className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleStrike().run()}
        isActive={editor.isActive('strike')}
        tooltip="删除线"
      >
        <Strikethrough className="h-4 w-4" />
      </ToolbarButton>

      <ToolbarButton
        onClick={() => editor.chain().focus().toggleHighlight().run()}
        isActive={editor.isActive('highlight')}
        tooltip="高亮"
      >
        <Highlighter className="h-4 w-4" />
      </ToolbarButton>
    </div>
  );
}
```

---

## 4. 主编辑器组件

### 4.1 完整代码 - NovelEditor.tsx

```tsx
import { useEffect, useCallback } from 'react';
import { useEditor, EditorContent } from '@tiptap/react';
import Document from '@tiptap/extension-document';
import Text from '@tiptap/extension-text';
import Paragraph from '@tiptap/extension-paragraph';
import Heading from '@tiptap/extension-heading';
import Blockquote from '@tiptap/extension-blockquote';
import HardBreak from '@tiptap/extension-hard-break';
import Bold from '@tiptap/extension-bold';
import Italic from '@tiptap/extension-italic';
import Strike from '@tiptap/extension-strike';
import Highlight from '@tiptap/extension-highlight';
import History from '@tiptap/extension-history';
import { UniqueIdExtension } from './extensions/unique-id';
import { EditorToolbar } from './EditorToolbar';
import { useEditorStore } from '@/stores/editor-store';
import { useAutoSave } from '@/hooks/use-auto-save';
import './editor.css';

interface NovelEditorProps {
  initialContent?: any;
  onContentChange?: (content: any) => void;
}

export function NovelEditor({ initialContent, onContentChange }: NovelEditorProps) {
  const { setEditor, setIsDirty } = useEditorStore();

  const editor = useEditor({
    extensions: [
      Document,
      Text,
      Paragraph.configure({
        HTMLAttributes: { class: 'novel-paragraph' },
      }),
      Heading.configure({
        levels: [1, 2, 3],
        HTMLAttributes: { class: 'novel-heading' },
      }),
      Blockquote.configure({
        HTMLAttributes: { class: 'novel-blockquote' },
      }),
      HardBreak,
      Bold,
      Italic,
      Strike,
      Highlight.configure({
        multicolor: true,
        HTMLAttributes: { class: 'novel-highlight' },
      }),
      History,
      UniqueIdExtension,
    ],
    content: initialContent || {
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [],
        },
      ],
    },
    onUpdate: ({ editor }) => {
      setIsDirty(true);
      onContentChange?.(editor.getJSON());
    },
    editorProps: {
      attributes: {
        class: 'novel-editor-content prose prose-sm focus:outline-none',
      },
    },
  });

  useEffect(() => {
    if (editor) {
      setEditor(editor);
    }
    return () => {
      setEditor(null);
    };
  }, [editor, setEditor]);

  useEffect(() => {
    if (editor && initialContent) {
      const currentContent = JSON.stringify(editor.getJSON());
      const newContent = JSON.stringify(initialContent);
      if (currentContent !== newContent) {
        editor.commands.setContent(initialContent);
        setIsDirty(false);
      }
    }
  }, [editor, initialContent, setIsDirty]);

  useAutoSave(editor);

  return (
    <div className="flex h-full flex-col">
      <EditorToolbar editor={editor} />
      <div className="flex-1 overflow-auto p-4">
        <EditorContent editor={editor} className="h-full" />
      </div>
    </div>
  );
}
```

---

## 5. 自动保存 Hook

### 5.1 完整代码 - hooks/use-auto-save.ts

```typescript
import { useEffect, useRef, useCallback } from 'react';
import { Editor } from '@tiptap/react';
import debounce from 'lodash.debounce';
import { useEditorStore } from '@/stores/editor-store';

const AUTO_SAVE_DELAY = 2000;
const MAX_SAVE_INTERVAL = 30000;

export function useAutoSave(editor: Editor | null) {
  const { 
    currentChapterPath, 
    isDirty, 
    saveCurrentChapter,
    setIsDirty,
  } = useEditorStore();
  
  const lastSaveTime = useRef<number>(Date.now());

  const debouncedSave = useCallback(
    debounce(async () => {
      if (isDirty && currentChapterPath) {
        await saveCurrentChapter();
        lastSaveTime.current = Date.now();
      }
    }, AUTO_SAVE_DELAY),
    [isDirty, currentChapterPath, saveCurrentChapter]
  );

  useEffect(() => {
    if (!editor) return;

    const handleUpdate = () => {
      debouncedSave();
    };

    editor.on('update', handleUpdate);

    return () => {
      editor.off('update', handleUpdate);
      debouncedSave.cancel();
    };
  }, [editor, debouncedSave]);

  useEffect(() => {
    const interval = setInterval(() => {
      const now = Date.now();
      if (isDirty && now - lastSaveTime.current >= MAX_SAVE_INTERVAL) {
        saveCurrentChapter();
        lastSaveTime.current = now;
      }
    }, 10000);

    return () => clearInterval(interval);
  }, [isDirty, saveCurrentChapter]);

  useEffect(() => {
    const handleBlur = () => {
      if (isDirty) {
        debouncedSave.cancel();
        saveCurrentChapter();
        lastSaveTime.current = Date.now();
      }
    };

    window.addEventListener('blur', handleBlur);
    return () => window.removeEventListener('blur', handleBlur);
  }, [isDirty, saveCurrentChapter, debouncedSave]);

  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (isDirty) {
        e.preventDefault();
        e.returnValue = '';
        saveCurrentChapter();
      }
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    return () => window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [isDirty, saveCurrentChapter]);
}
```

---

## 6. 字数统计 Hook

### 6.1 完整代码 - hooks/use-word-count.ts

```typescript
import { useMemo } from 'react';
import { Editor } from '@tiptap/react';

interface WordCountResult {
  textLengthNoWhitespace: number;
  wordCount: number;
  paragraphCount: number;
}

export function useWordCount(editor: Editor | null): WordCountResult {
  return useMemo(() => {
    if (!editor) {
      return {
        textLengthNoWhitespace: 0,
        wordCount: 0,
        paragraphCount: 0,
      };
    }

    const json = editor.getJSON();
    const plainText = extractPlainText(json);
    
    const textLengthNoWhitespace = plainText.replace(/\s/g, '').length;
    
    const wordCount = plainText
      .split(/\s+/)
      .filter((word) => word.length > 0).length;
    
    let paragraphCount = 0;
    if (json.content) {
      paragraphCount = json.content.filter(
        (node: any) => node.type === 'paragraph' && node.content?.length > 0
      ).length;
    }

    return {
      textLengthNoWhitespace,
      wordCount,
      paragraphCount,
    };
  }, [editor?.state.doc]);
}

function extractPlainText(doc: any): string {
  const texts: string[] = [];

  function walk(node: any) {
    if (node.type === 'text' && node.text) {
      texts.push(node.text);
    } else if (node.content) {
      node.content.forEach(walk);
    }
  }

  walk(doc);
  return texts.join('');
}
```

---

## 7. 编辑器状态管理

### 7.1 完整代码 - stores/editor-store.ts

```typescript
import { create } from 'zustand';
import { Editor } from '@tiptap/react';
import { commands } from '@/lib/tauri-commands';

interface EditorState {
  editor: Editor | null;
  projectRoot: string | null;
  libraryRoot: string | null;
  currentChapterPath: string | null;
  currentChapterId: string | null;
  currentChapterTitle: string | null;
  isSaving: boolean;
  isDirty: boolean;
  lastSavedAt: number | null;

  setEditor: (editor: Editor | null) => void;
  setProjectRoot: (root: string | null) => void;
  setLibraryRoot: (root: string | null) => void;
  setIsDirty: (dirty: boolean) => void;
  
  loadChapter: (path: string) => Promise<void>;
  saveCurrentChapter: () => Promise<void>;
  closeChapter: () => void;
  
  applyPatch: (patch: any) => void;
}

export const useEditorStore = create<EditorState>((set, get) => ({
  editor: null,
  projectRoot: null,
  libraryRoot: null,
  currentChapterPath: null,
  currentChapterId: null,
  currentChapterTitle: null,
  isSaving: false,
  isDirty: false,
  lastSavedAt: null,

  setEditor: (editor) => set({ editor }),
  setProjectRoot: (root) => set({ projectRoot: root }),
  setLibraryRoot: (root) => set({ libraryRoot: root }),
  setIsDirty: (dirty) => set({ isDirty: dirty }),

  loadChapter: async (path) => {
    const { projectRoot, editor } = get();
    if (!projectRoot || !editor) return;

    try {
      const chapter = await commands.readChapter(projectRoot, path);
      
      editor.commands.setContent(chapter.content);
      
      if (chapter.last_cursor_position) {
        try {
          editor.commands.setTextSelection(chapter.last_cursor_position);
        } catch {
        }
      }

      set({
        currentChapterPath: path,
        currentChapterId: chapter.id,
        currentChapterTitle: chapter.title,
        isDirty: false,
      });
    } catch (error) {
      console.error('Failed to load chapter:', error);
      throw error;
    }
  },

  saveCurrentChapter: async () => {
    const { projectRoot, currentChapterPath, editor, isSaving, isDirty } = get();
    
    if (!projectRoot || !currentChapterPath || !editor || isSaving || !isDirty) {
      return;
    }

    set({ isSaving: true });

    try {
      const existingChapter = await commands.readChapter(projectRoot, currentChapterPath);
      
      const cursorPosition = editor.state.selection.anchor;
      
      const updatedChapter = {
        ...existingChapter,
        content: editor.getJSON(),
        last_cursor_position: cursorPosition,
      };

      await commands.saveChapter(projectRoot, currentChapterPath, updatedChapter);

      set({
        isDirty: false,
        isSaving: false,
        lastSavedAt: Date.now(),
      });
    } catch (error) {
      console.error('Failed to save chapter:', error);
      set({ isSaving: false });
      throw error;
    }
  },

  closeChapter: () => {
    const { editor } = get();
    if (editor) {
      editor.commands.setContent({
        type: 'doc',
        content: [{ type: 'paragraph', content: [] }],
      });
    }
    set({
      currentChapterPath: null,
      currentChapterId: null,
      currentChapterTitle: null,
      isDirty: false,
    });
  },

  applyPatch: (patch) => {
    const { editor } = get();
    if (!editor) return;

    for (const op of patch) {
      switch (op.op) {
        case 'insert_blocks':
          applyInsertBlocks(editor, op.after_block_id, op.blocks);
          break;
        case 'update_block':
          applyUpdateBlock(editor, op.block_id, op.after);
          break;
        case 'delete_blocks':
          applyDeleteBlocks(editor, op.block_ids);
          break;
      }
    }

    set({ isDirty: true });
  },
}));

function applyInsertBlocks(editor: Editor, afterBlockId: string | null, blocks: any[]) {
  const { state } = editor;
  let insertPos = 0;

  if (afterBlockId) {
    state.doc.descendants((node, pos) => {
      if (node.attrs.id === afterBlockId) {
        insertPos = pos + node.nodeSize;
        return false;
      }
    });
  }

  editor.chain().focus().insertContentAt(insertPos, blocks).run();
}

function applyUpdateBlock(editor: Editor, blockId: string, newContent: any) {
  const { state } = editor;
  
  state.doc.descendants((node, pos) => {
    if (node.attrs.id === blockId) {
      editor.chain()
        .focus()
        .setNodeSelection(pos)
        .deleteSelection()
        .insertContentAt(pos, { ...newContent, attrs: { ...newContent.attrs, id: blockId } })
        .run();
      return false;
    }
  });
}

function applyDeleteBlocks(editor: Editor, blockIds: string[]) {
  const { state } = editor;
  const tr = state.tr;
  const positionsToDelete: { from: number; to: number }[] = [];

  state.doc.descendants((node, pos) => {
    if (blockIds.includes(node.attrs.id)) {
      positionsToDelete.push({ from: pos, to: pos + node.nodeSize });
    }
  });

  positionsToDelete
    .sort((a, b) => b.from - a.from)
    .forEach(({ from, to }) => {
      tr.delete(from, to);
    });

  editor.view.dispatch(tr);
}
```

---

## 8. 编辑器样式

### 8.1 完整代码 - styles/editor.css

```css
.novel-editor-content {
  min-height: 100%;
  max-width: 800px;
  margin: 0 auto;
  padding: 2rem;
}

.novel-paragraph {
  margin-bottom: 1em;
  line-height: 1.8;
  text-indent: 2em;
}

.novel-heading {
  font-weight: 600;
  margin-top: 1.5em;
  margin-bottom: 0.5em;
}

.novel-heading[data-level="1"] {
  font-size: 1.75rem;
}

.novel-heading[data-level="2"] {
  font-size: 1.5rem;
}

.novel-heading[data-level="3"] {
  font-size: 1.25rem;
}

.novel-blockquote {
  border-left: 3px solid #e5e7eb;
  padding-left: 1rem;
  margin: 1rem 0;
  color: #6b7280;
  font-style: italic;
}

.novel-highlight {
  background-color: #fef08a;
  padding: 0.1em 0.2em;
  border-radius: 0.2em;
}

.novel-highlight[data-color="ai-new"] {
  background-color: #bbf7d0;
}

.novel-highlight[data-color="ai-modified"] {
  background-color: #fef08a;
}

.novel-highlight[data-color="ai-pending"] {
  background-color: #e9d5ff;
}

.ProseMirror {
  outline: none;
}

.ProseMirror p.is-editor-empty:first-child::before {
  content: "开始写作...";
  color: #9ca3af;
  float: left;
  pointer-events: none;
  height: 0;
}

.ProseMirror-selectednode {
  outline: 2px solid #3b82f6;
}
```

---

## 9. Zod Schema 定义

### 9.1 完整代码 - lib/schemas.ts

```typescript
import { z } from 'zod';

export const TiptapMarkSchema = z.union([
  z.object({ type: z.literal('bold') }),
  z.object({ type: z.literal('italic') }),
  z.object({ type: z.literal('strike') }),
  z.object({ 
    type: z.literal('highlight'), 
    attrs: z.object({ color: z.string() }).optional() 
  }),
]);

export const TiptapInlineSchema: z.ZodType<any> = z.union([
  z.object({
    type: z.literal('text'),
    text: z.string(),
    marks: z.array(TiptapMarkSchema).optional(),
  }),
  z.object({ type: z.literal('hardBreak') }),
]);

export const TiptapBlockSchema: z.ZodType<any> = z.union([
  z.object({
    type: z.literal('paragraph'),
    attrs: z.object({ id: z.string() }),
    content: z.array(TiptapInlineSchema).optional(),
  }),
  z.object({
    type: z.literal('heading'),
    attrs: z.object({ 
      id: z.string(),
      level: z.union([z.literal(1), z.literal(2), z.literal(3)]),
    }),
    content: z.array(TiptapInlineSchema).optional(),
  }),
  z.object({
    type: z.literal('blockquote'),
    attrs: z.object({ id: z.string() }),
    content: z.lazy(() => z.array(TiptapBlockSchema)).optional(),
  }),
]);

export const TiptapDocSchema = z.object({
  type: z.literal('doc'),
  content: z.array(TiptapBlockSchema),
});

export const ChapterCountsSchema = z.object({
  text_length_no_whitespace: z.number(),
  word_count: z.number().optional(),
  algorithm_version: z.number(),
  last_calculated_at: z.number(),
});

export const ChapterSchema = z.object({
  schema_version: z.number(),
  id: z.string(),
  title: z.string(),
  content: TiptapDocSchema,
  counts: ChapterCountsSchema,
  target_words: z.number().optional(),
  status: z.enum(['draft', 'revised', 'final']).optional(),
  summary: z.string().optional(),
  tags: z.array(z.string()).optional(),
  last_cursor_position: z.number().optional(),
  created_at: z.number(),
  updated_at: z.number(),
});

export type TiptapDoc = z.infer<typeof TiptapDocSchema>;
export type TiptapBlock = z.infer<typeof TiptapBlockSchema>;
export type Chapter = z.infer<typeof ChapterSchema>;
```

---

## 10. Tauri 命令封装

### 10.1 完整代码 - lib/tauri-commands.ts

```typescript
import { invoke } from '@tauri-apps/api/core';

export const commands = {
  setLibraryRoot: (path: string) =>
    invoke<void>('set_library_root', { path }),

  createProject: (libraryRoot: string, projectFolderName: string, name: string, author: string) =>
    invoke<string>('create_project', { libraryRoot, projectFolderName, name, author }),

  openProject: (projectRoot: string) =>
    invoke<any>('open_project', { projectRoot }),

  createVolume: (projectRoot: string, parentPath: string, folderName: string, title: string) =>
    invoke<string>('create_volume', { projectRoot, parentPath, folderName, title }),

  deleteVolume: (projectRoot: string, path: string) =>
    invoke<void>('delete_volume', { projectRoot, path }),

  renameVolume: (projectRoot: string, oldPath: string, newName: string) =>
    invoke<string>('rename_volume', { projectRoot, oldPath, newName }),

  createChapter: (projectRoot: string, volumePath: string, fileName: string, title: string) =>
    invoke<string>('create_chapter', { projectRoot, volumePath, fileName, title }),

  readChapter: (projectRoot: string, path: string) =>
    invoke<any>('read_chapter', { projectRoot, path }),

  saveChapter: (projectRoot: string, path: string, data: any) =>
    invoke<void>('save_chapter', { projectRoot, path, data }),

  deleteChapter: (projectRoot: string, path: string) =>
    invoke<void>('delete_chapter', { projectRoot, path }),

  renameChapter: (projectRoot: string, oldPath: string, newName: string) =>
    invoke<string>('rename_chapter', { projectRoot, oldPath, newName }),

  saveAiProposal: (projectRoot: string, proposal: any) =>
    invoke<void>('save_ai_proposal', { projectRoot, proposal }),

  appendChapterHistoryEvent: (projectRoot: string, chapterId: string, event: any) =>
    invoke<void>('append_chapter_history_event', { projectRoot, chapterId, event }),
};
```

---

## 下一步

编辑器完成后，继续阅读 [04_ui_components.md](./04_ui_components.md) 开始 UI 组件开发。

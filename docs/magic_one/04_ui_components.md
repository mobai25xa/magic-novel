# UI 组件开发文档

> 本文档详细描述应用的 UI 布局和组件实现。

---

## 1. 整体布局

```
┌─────────────────────────────────────────────────────────────────┐
│                          TopBar                                  │
├──────────────┬────────────────────────────────┬─────────────────┤
│              │                                │                 │
│  LeftPanel   │        EditorPanel             │   RightPanel    │
│  (250px)     │        (flex: 1)               │   (300px)       │
│              │                                │                 │
│  ┌────────┐  │  ┌──────────────────────────┐  │  ┌───────────┐  │
│  │ Tree   │  │  │     EditorToolbar        │  │  │  AI 助手  │  │
│  │        │  │  ├──────────────────────────┤  │  │  即将推出 │  │
│  │        │  │  │                          │  │  │           │  │
│  │        │  │  │     NovelEditor          │  │  │           │  │
│  │        │  │  │                          │  │  │           │  │
│  └────────┘  │  │                          │  │  │           │  │
│  ┌────────┐  │  │                          │  │  │           │  │
│  │ Info   │  │  │                          │  │  │           │  │
│  │ Panel  │  │  │                          │  │  │           │  │
│  └────────┘  │  └──────────────────────────┘  │  └───────────┘  │
└──────────────┴────────────────────────────────┴─────────────────┘
```

---

## 2. 主应用入口

### 2.1 App.tsx

```tsx
import { useEffect, useState } from 'react';
import { TooltipProvider } from '@/components/ui/tooltip';
import { TopBar } from '@/components/layout/TopBar';
import { LeftPanel } from '@/components/layout/LeftPanel';
import { EditorPanel } from '@/components/layout/EditorPanel';
import { RightPanel } from '@/components/layout/RightPanel';
import { useProjectStore } from '@/stores/project-store';
import { WelcomeDialog } from '@/components/dialogs/WelcomeDialog';

export default function App() {
  const { projectRoot, isLoading } = useProjectStore();
  const [showWelcome, setShowWelcome] = useState(!projectRoot);

  useEffect(() => {
    if (!projectRoot) {
      setShowWelcome(true);
    }
  }, [projectRoot]);

  return (
    <TooltipProvider>
      <div className="flex h-screen flex-col bg-background text-foreground">
        <TopBar />
        <div className="flex flex-1 overflow-hidden">
          <LeftPanel />
          <EditorPanel />
          <RightPanel />
        </div>
      </div>
      <WelcomeDialog open={showWelcome} onOpenChange={setShowWelcome} />
    </TooltipProvider>
  );
}
```

---

## 3. 顶部栏

### 3.1 TopBar.tsx

```tsx
import { Settings, FolderOpen, FilePlus, Save } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useProjectStore } from '@/stores/project-store';
import { useEditorStore } from '@/stores/editor-store';

export function TopBar() {
  const { projectName, openProjectDialog, createProjectDialog } = useProjectStore();
  const { isDirty, isSaving, saveCurrentChapter, currentChapterTitle } = useEditorStore();

  return (
    <header className="flex h-12 items-center justify-between border-b bg-background px-4">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          <img src="/magic.png" alt="Magic Novel" className="h-6 w-6" />
          <span className="font-semibold">Magic Novel</span>
        </div>
        
        {projectName && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <span>/</span>
            <span>{projectName}</span>
            {currentChapterTitle && (
              <>
                <span>/</span>
                <span>{currentChapterTitle}</span>
              </>
            )}
            {isDirty && <span className="text-orange-500">*</span>}
          </div>
        )}
      </div>

      <div className="flex items-center gap-2">
        {isDirty && (
          <Button
            variant="ghost"
            size="sm"
            onClick={saveCurrentChapter}
            disabled={isSaving}
          >
            <Save className="mr-2 h-4 w-4" />
            {isSaving ? '保存中...' : '保存'}
          </Button>
        )}

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="ghost" size="icon">
              <Settings className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem onClick={createProjectDialog}>
              <FilePlus className="mr-2 h-4 w-4" />
              新建作品
            </DropdownMenuItem>
            <DropdownMenuItem onClick={openProjectDialog}>
              <FolderOpen className="mr-2 h-4 w-4" />
              打开作品
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem>
              <Settings className="mr-2 h-4 w-4" />
              偏好设置
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </header>
  );
}
```

---

## 4. 左侧面板

### 4.1 LeftPanel.tsx

```tsx
import { ScrollArea } from '@/components/ui/scroll-area';
import { ContentTree } from '@/components/tree/ContentTree';
import { ChapterInfo } from '@/components/tree/ChapterInfo';
import { useProjectStore } from '@/stores/project-store';

export function LeftPanel() {
  const { projectRoot } = useProjectStore();

  if (!projectRoot) {
    return (
      <aside className="flex w-64 flex-col border-r bg-muted/30">
        <div className="flex flex-1 items-center justify-center p-4 text-sm text-muted-foreground">
          请先打开或创建作品
        </div>
      </aside>
    );
  }

  return (
    <aside className="flex w-64 flex-col border-r bg-muted/30">
      <div className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <ContentTree />
        </ScrollArea>
      </div>
      <ChapterInfo />
    </aside>
  );
}
```

### 4.2 ContentTree.tsx

```tsx
import { useState } from 'react';
import { 
  ChevronRight, 
  ChevronDown, 
  Folder, 
  FileText,
  Plus,
  MoreHorizontal,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from '@/components/ui/context-menu';
import { cn } from '@/lib/utils';
import { useProjectStore } from '@/stores/project-store';
import { useEditorStore } from '@/stores/editor-store';

interface TreeNodeProps {
  node: any;
  level: number;
}

function TreeNode({ node, level }: TreeNodeProps) {
  const [isExpanded, setIsExpanded] = useState(true);
  const { loadChapter, currentChapterPath } = useEditorStore();
  const { createChapterDialog, createVolumeDialog, deleteNode, renameNode } = useProjectStore();

  const isDir = node.kind === 'dir';
  const isSelected = node.path === currentChapterPath;

  const handleClick = () => {
    if (isDir) {
      setIsExpanded(!isExpanded);
    } else {
      loadChapter(node.path);
    }
  };

  return (
    <ContextMenu>
      <ContextMenuTrigger>
        <div>
          <div
            className={cn(
              'group flex cursor-pointer items-center gap-1 rounded-md px-2 py-1 hover:bg-accent',
              isSelected && 'bg-accent'
            )}
            style={{ paddingLeft: `${level * 12 + 8}px` }}
            onClick={handleClick}
          >
            {isDir ? (
              <>
                {isExpanded ? (
                  <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
                ) : (
                  <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
                )}
                <Folder className="h-4 w-4 shrink-0 text-yellow-500" />
              </>
            ) : (
              <>
                <span className="w-4" />
                <FileText className="h-4 w-4 shrink-0 text-blue-500" />
              </>
            )}
            <span className="flex-1 truncate text-sm">
              {isDir ? node.name : node.title}
            </span>
            {!isDir && (
              <span className="text-xs text-muted-foreground">
                {node.text_length_no_whitespace}字
              </span>
            )}
          </div>

          {isDir && isExpanded && node.children && (
            <div>
              {node.children.map((child: any) => (
                <TreeNode key={child.path} node={child} level={level + 1} />
              ))}
            </div>
          )}
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        {isDir && (
          <>
            <ContextMenuItem onClick={() => createChapterDialog(node.path)}>
              新建章节
            </ContextMenuItem>
            <ContextMenuItem onClick={() => createVolumeDialog(node.path)}>
              新建子卷
            </ContextMenuItem>
          </>
        )}
        <ContextMenuItem onClick={() => renameNode(node.path, isDir)}>
          重命名
        </ContextMenuItem>
        <ContextMenuItem 
          onClick={() => deleteNode(node.path, isDir)}
          className="text-destructive"
        >
          删除
        </ContextMenuItem>
      </ContextMenuContent>
    </ContextMenu>
  );
}

export function ContentTree() {
  const { tree, createVolumeDialog } = useProjectStore();

  return (
    <div className="p-2">
      <div className="mb-2 flex items-center justify-between px-2">
        <span className="text-xs font-medium text-muted-foreground">目录</span>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => createVolumeDialog('content')}
        >
          <Plus className="h-3 w-3" />
        </Button>
      </div>

      {tree.length === 0 ? (
        <div className="px-2 py-4 text-center text-sm text-muted-foreground">
          暂无内容，点击 + 创建卷
        </div>
      ) : (
        tree.map((node) => <TreeNode key={node.path} node={node} level={0} />)
      )}
    </div>
  );
}
```

### 4.3 ChapterInfo.tsx

```tsx
import { useEditorStore } from '@/stores/editor-store';
import { useWordCount } from '@/hooks/use-word-count';
import { Separator } from '@/components/ui/separator';

export function ChapterInfo() {
  const { editor, currentChapterTitle, lastSavedAt } = useEditorStore();
  const { textLengthNoWhitespace, paragraphCount } = useWordCount(editor);

  if (!currentChapterTitle) {
    return null;
  }

  const formatTime = (timestamp: number | null) => {
    if (!timestamp) return '-';
    return new Date(timestamp).toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <div className="border-t bg-muted/50 p-3">
      <div className="mb-2 text-xs font-medium text-muted-foreground">章节信息</div>
      <div className="space-y-1 text-sm">
        <div className="flex justify-between">
          <span className="text-muted-foreground">字数</span>
          <span>{textLengthNoWhitespace.toLocaleString()}</span>
        </div>
        <div className="flex justify-between">
          <span className="text-muted-foreground">段落</span>
          <span>{paragraphCount}</span>
        </div>
        <Separator className="my-2" />
        <div className="flex justify-between">
          <span className="text-muted-foreground">上次保存</span>
          <span>{formatTime(lastSavedAt)}</span>
        </div>
      </div>
    </div>
  );
}
```

---

## 5. 编辑器面板

### 5.1 EditorPanel.tsx

```tsx
import { NovelEditor } from '@/components/editor/NovelEditor';
import { useEditorStore } from '@/stores/editor-store';

export function EditorPanel() {
  const { currentChapterPath } = useEditorStore();

  if (!currentChapterPath) {
    return (
      <main className="flex flex-1 items-center justify-center bg-background">
        <div className="text-center text-muted-foreground">
          <p className="text-lg">选择一个章节开始编辑</p>
          <p className="mt-2 text-sm">或从左侧目录创建新章节</p>
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden bg-background">
      <NovelEditor />
    </main>
  );
}
```

---

## 6. 右侧面板（AI 占位）

### 6.1 RightPanel.tsx

```tsx
import { Bot, Sparkles } from 'lucide-react';

export function RightPanel() {
  return (
    <aside className="flex w-72 flex-col border-l bg-muted/30">
      <div className="border-b p-3">
        <div className="flex items-center gap-2">
          <Bot className="h-4 w-4" />
          <span className="text-sm font-medium">AI 助手</span>
        </div>
      </div>

      <div className="flex flex-1 flex-col items-center justify-center p-6">
        <div className="mb-4 rounded-full bg-primary/10 p-4">
          <Sparkles className="h-8 w-8 text-primary" />
        </div>
        <h3 className="mb-2 text-lg font-medium">AI 助手即将推出</h3>
        <p className="text-center text-sm text-muted-foreground">
          智能写作助手正在开发中，敬请期待！
        </p>
        <div className="mt-6 space-y-2 text-center text-xs text-muted-foreground">
          <p>即将支持：</p>
          <ul className="space-y-1">
            <li>• 智能续写</li>
            <li>• 情节建议</li>
            <li>• 角色对话生成</li>
            <li>• 润色修改</li>
          </ul>
        </div>
      </div>
    </aside>
  );
}
```

---

## 7. 项目状态管理

### 7.1 project-store.ts

```typescript
import { create } from 'zustand';
import { open } from '@tauri-apps/plugin-dialog';
import { commands } from '@/lib/tauri-commands';

interface ProjectState {
  projectRoot: string | null;
  projectName: string | null;
  tree: any[];
  isLoading: boolean;

  openProjectDialog: () => Promise<void>;
  createProjectDialog: () => Promise<void>;
  loadProject: (projectRoot: string) => Promise<void>;
  refreshTree: () => Promise<void>;
  
  createVolumeDialog: (parentPath: string) => void;
  createChapterDialog: (volumePath: string) => void;
  deleteNode: (path: string, isDir: boolean) => Promise<void>;
  renameNode: (path: string, isDir: boolean) => void;
}

export const useProjectStore = create<ProjectState>((set, get) => ({
  projectRoot: null,
  projectName: null,
  tree: [],
  isLoading: false,

  openProjectDialog: async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: '选择作品目录',
    });

    if (selected && typeof selected === 'string') {
      await get().loadProject(selected);
    }
  },

  createProjectDialog: async () => {
    const libraryRoot = await open({
      directory: true,
      multiple: false,
      title: '选择作品库目录',
    });

    if (!libraryRoot || typeof libraryRoot !== 'string') {
      return;
    }

    const projectName = prompt('请输入作品名称：');
    if (!projectName) return;

    const author = prompt('请输入作者名：') || '匿名';
    const folderName = projectName.replace(/[<>:"/\\|?*]/g, '_');

    try {
      const projectRoot = await commands.createProject(
        libraryRoot,
        folderName,
        projectName,
        author
      );
      await get().loadProject(projectRoot);
    } catch (error) {
      console.error('Failed to create project:', error);
      alert('创建作品失败：' + (error as any).message);
    }
  },

  loadProject: async (projectRoot) => {
    set({ isLoading: true });

    try {
      const snapshot = await commands.openProject(projectRoot);
      set({
        projectRoot,
        projectName: snapshot.project.name,
        tree: snapshot.tree,
        isLoading: false,
      });
    } catch (error) {
      console.error('Failed to load project:', error);
      set({ isLoading: false });
      throw error;
    }
  },

  refreshTree: async () => {
    const { projectRoot } = get();
    if (!projectRoot) return;

    try {
      const snapshot = await commands.openProject(projectRoot);
      set({ tree: snapshot.tree });
    } catch (error) {
      console.error('Failed to refresh tree:', error);
    }
  },

  createVolumeDialog: (parentPath) => {
    const title = prompt('请输入卷名：');
    if (!title) return;

    const folderName = title.replace(/[<>:"/\\|?*]/g, '_');
    const { projectRoot, refreshTree } = get();
    
    if (projectRoot) {
      commands.createVolume(projectRoot, parentPath, folderName, title)
        .then(() => refreshTree())
        .catch((error) => {
          console.error('Failed to create volume:', error);
          alert('创建卷失败');
        });
    }
  },

  createChapterDialog: (volumePath) => {
    const title = prompt('请输入章节标题：');
    if (!title) return;

    const count = get().tree.length + 1;
    const fileName = `${String(count).padStart(3, '0')}_${title.replace(/[<>:"/\\|?*]/g, '_')}`;
    const { projectRoot, refreshTree } = get();
    
    if (projectRoot) {
      commands.createChapter(projectRoot, volumePath, fileName, title)
        .then(() => refreshTree())
        .catch((error) => {
          console.error('Failed to create chapter:', error);
          alert('创建章节失败');
        });
    }
  },

  deleteNode: async (path, isDir) => {
    const confirmed = confirm(`确定要删除${isDir ? '此卷及其所有章节' : '此章节'}吗？`);
    if (!confirmed) return;

    const { projectRoot, refreshTree } = get();
    if (!projectRoot) return;

    try {
      if (isDir) {
        await commands.deleteVolume(projectRoot, path);
      } else {
        await commands.deleteChapter(projectRoot, path);
      }
      await refreshTree();
    } catch (error) {
      console.error('Failed to delete:', error);
      alert('删除失败');
    }
  },

  renameNode: (path, isDir) => {
    const newName = prompt('请输入新名称：');
    if (!newName) return;

    const { projectRoot, refreshTree } = get();
    if (!projectRoot) return;

    const promise = isDir
      ? commands.renameVolume(projectRoot, path, newName)
      : commands.renameChapter(projectRoot, path, newName.replace(/[<>:"/\\|?*]/g, '_'));

    promise
      .then(() => refreshTree())
      .catch((error) => {
        console.error('Failed to rename:', error);
        alert('重命名失败');
      });
  },
}));
```

---

## 8. 欢迎对话框

### 8.1 WelcomeDialog.tsx

```tsx
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { FolderOpen, FilePlus } from 'lucide-react';
import { useProjectStore } from '@/stores/project-store';

interface WelcomeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function WelcomeDialog({ open, onOpenChange }: WelcomeDialogProps) {
  const { openProjectDialog, createProjectDialog } = useProjectStore();

  const handleOpen = async () => {
    await openProjectDialog();
    onOpenChange(false);
  };

  const handleCreate = async () => {
    await createProjectDialog();
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <img src="/magic.png" alt="Magic Novel" className="h-10 w-10" />
            <div>
              <DialogTitle>欢迎使用 Magic Novel</DialogTitle>
              <DialogDescription>开始你的创作之旅</DialogDescription>
            </div>
          </div>
        </DialogHeader>
        <div className="mt-4 flex flex-col gap-3">
          <Button onClick={handleCreate} className="justify-start gap-2">
            <FilePlus className="h-4 w-4" />
            新建作品
          </Button>
          <Button variant="outline" onClick={handleOpen} className="justify-start gap-2">
            <FolderOpen className="h-4 w-4" />
            打开已有作品
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
```

---

## 9. 安装 Dialog 插件

```bash
pnpm add @tauri-apps/plugin-dialog
```

在 `src-tauri/Cargo.toml` 添加：

```toml
tauri-plugin-dialog = "2"
```

在 `src-tauri/src/lib.rs` 中注册：

```rust
tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    // ...
```

---

## 下一步

UI 组件完成后，继续阅读 [05_import_export.md](./05_import_export.md) 开始导入导出功能开发。

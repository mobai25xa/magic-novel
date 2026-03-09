# AI 基础设施文档

> 本文档详细描述 AI 写作助手的底座设计，为 Phase 2 AI 功能做准备。

---

## 1. 架构概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        用户界面 (Phase 2)                        │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────────┐   │
│  │  AI 聊天面板  │  │  内联建议UI   │  │   Diff 审阅视图   │   │
│  └───────────────┘  └───────────────┘  └───────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                      前端 AI Hooks (Phase 1)                     │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ getCurrentChapterContent / insertBlocksAfter / applyPatch │  │
│  └───────────────────────────────────────────────────────────┘  │
├─────────────────────────────────────────────────────────────────┤
│                    Tauri Commands (Phase 1)                      │
│  ┌─────────────────────┐  ┌─────────────────────────────────┐   │
│  │  save_ai_proposal   │  │  append_chapter_history_event   │   │
│  └─────────────────────┘  └─────────────────────────────────┘   │
├─────────────────────────────────────────────────────────────────┤
│                     文件存储 (Phase 1)                           │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │  magic_novel/ai/proposals/{id}.json                         ││
│  │  magic_novel/history/chapters/{id}.jsonl                    ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Phase 1 必须完成的底座

### 2.1 数据存储

| 存储 | 路径 | 格式 | 用途 |
|------|------|------|------|
| Proposal | `magic_novel/ai/proposals/{id}.json` | JSON | AI 候选内容 |
| History | `magic_novel/history/chapters/{id}.jsonl` | JSONL | 变更历史 |

### 2.2 Rust 命令

| 命令 | 功能 |
|------|------|
| `save_ai_proposal` | 保存 AI 生成的候选稿 |
| `append_chapter_history_event` | 追加章节变更事件 |

### 2.3 前端能力

| 能力 | 功能 |
|------|------|
| `applyPatch` | 执行 insert/update/delete 操作 |
| `highlightBlocks` | 高亮 AI 生成的内容 |

---

## 3. Proposal 工作流

### 3.1 流程图

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  用户    │────▶│  生成    │────▶│  预览    │────▶│  应用    │
│  请求    │     │  Proposal│     │  候选稿  │     │  到正文  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
                      │                │                │
                      ▼                ▼                ▼
                 保存 proposal    显示 diff        记录 history
                 (generated)     高亮显示         更新状态
```

### 3.2 Proposal 生命周期

```typescript
type ProposalStatus = 
  | "generated"          // AI 生成完成，待用户审阅
  | "accepted"           // 用户完全接受
  | "partially_accepted" // 用户部分接受
  | "rejected";          // 用户拒绝
```

### 3.3 前端 Proposal 管理

```typescript
interface ProposalManager {
  currentProposal: AiProposal | null;
  
  createProposal: (params: CreateProposalParams) => Promise<AiProposal>;
  
  previewProposal: (proposal: AiProposal) => void;
  
  acceptProposal: (proposal: AiProposal) => Promise<void>;
  
  rejectProposal: (proposal: AiProposal) => Promise<void>;
  
  partialAccept: (proposal: AiProposal, selectedBlockIds: string[]) => Promise<void>;
}

interface CreateProposalParams {
  chapterId: string;
  chapterPath: string;
  target: {
    type: "cursor" | "block";
    blockId?: string;
    position?: "before" | "after" | "replace";
  };
  prompt: string;
  contextRefs?: {
    loreAssetIds?: string[];
    promptAssetIds?: string[];
  };
}
```

---

## 4. History 事件设计

### 4.1 事件类型

```typescript
interface ChapterHistoryEvent {
  schema_version: number;
  event_id: string;
  created_at: number;
  
  actor: "human" | "ai";
  source_proposal_id?: string;
  
  before_hash: string;
  after_hash: string;
  
  summary?: string;
  patch: PatchOp[];
}
```

### 4.2 Patch 操作

```typescript
type PatchOp =
  | {
      op: "insert_blocks";
      after_block_id: string | null;
      blocks: TiptapBlock[];
    }
  | {
      op: "update_block";
      block_id: string;
      before: TiptapBlock;
      after: TiptapBlock;
    }
  | {
      op: "delete_blocks";
      block_ids: string[];
    };
```

### 4.3 Hash 计算

```typescript
function calculateDocHash(doc: TiptapDoc): string {
  const content = JSON.stringify(doc);
  return crypto.subtle.digest('SHA-256', new TextEncoder().encode(content))
    .then(buffer => Array.from(new Uint8Array(buffer))
      .map(b => b.toString(16).padStart(2, '0'))
      .join(''));
}
```

---

## 5. 前端 AI Hooks 实现

### 5.1 hooks/use-ai-agent.ts

```typescript
import { useCallback } from 'react';
import { useEditorStore } from '@/stores/editor-store';
import { commands } from '@/lib/tauri-commands';

export function useAiAgent() {
  const { editor, projectRoot, currentChapterId } = useEditorStore();

  const getCurrentChapterContent = useCallback(() => {
    if (!editor) return null;
    return editor.getJSON();
  }, [editor]);

  const getBlockById = useCallback((blockId: string) => {
    if (!editor) return null;
    
    let foundBlock = null;
    editor.state.doc.descendants((node) => {
      if (node.attrs.id === blockId) {
        foundBlock = node.toJSON();
        return false;
      }
    });
    return foundBlock;
  }, [editor]);

  const getSelectedBlocks = useCallback(() => {
    if (!editor) return [];
    
    const { from, to } = editor.state.selection;
    const blocks: any[] = [];
    
    editor.state.doc.nodesBetween(from, to, (node) => {
      if (node.attrs.id) {
        blocks.push(node.toJSON());
      }
    });
    
    return blocks;
  }, [editor]);

  const getCurrentWordCount = useCallback(() => {
    if (!editor) return 0;
    
    const json = editor.getJSON();
    const text = extractPlainText(json);
    return text.replace(/\s/g, '').length;
  }, [editor]);

  const insertBlocksAfter = useCallback((afterBlockId: string | null, blocks: any[]) => {
    if (!editor) return;

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
  }, [editor]);

  const replaceBlock = useCallback((blockId: string, newBlock: any) => {
    if (!editor) return;

    const { state } = editor;
    
    state.doc.descendants((node, pos) => {
      if (node.attrs.id === blockId) {
        editor.chain()
          .focus()
          .setNodeSelection(pos)
          .deleteSelection()
          .insertContentAt(pos, { ...newBlock, attrs: { ...newBlock.attrs, id: blockId } })
          .run();
        return false;
      }
    });
  }, [editor]);

  const deleteBlocks = useCallback((blockIds: string[]) => {
    if (!editor) return;

    const { state } = editor;
    const tr = state.tr;
    const positions: { from: number; to: number }[] = [];

    state.doc.descendants((node, pos) => {
      if (blockIds.includes(node.attrs.id)) {
        positions.push({ from: pos, to: pos + node.nodeSize });
      }
    });

    positions.sort((a, b) => b.from - a.from).forEach(({ from, to }) => {
      tr.delete(from, to);
    });

    editor.view.dispatch(tr);
  }, [editor]);

  const highlightBlocks = useCallback((blockIds: string[], color: string) => {
    if (!editor) return;

    blockIds.forEach(blockId => {
      editor.state.doc.descendants((node, pos) => {
        if (node.attrs.id === blockId && node.isTextblock) {
          const from = pos + 1;
          const to = pos + node.nodeSize - 1;
          editor.chain()
            .setTextSelection({ from, to })
            .setHighlight({ color })
            .run();
          return false;
        }
      });
    });
  }, [editor]);

  const clearAiHighlights = useCallback(() => {
    if (!editor) return;
    editor.chain().selectAll().unsetHighlight().run();
  }, [editor]);

  const recordHistoryEvent = useCallback(async (
    patch: any[],
    actor: "human" | "ai",
    proposalId?: string,
    summary?: string
  ) => {
    if (!projectRoot || !currentChapterId) return;

    const doc = editor?.getJSON();
    const afterHash = doc ? await calculateDocHash(doc) : '';

    const event = {
      schema_version: 1,
      event_id: crypto.randomUUID(),
      created_at: Date.now(),
      actor,
      source_proposal_id: proposalId,
      before_hash: '',
      after_hash: afterHash,
      summary,
      patch,
    };

    await commands.appendChapterHistoryEvent(projectRoot, currentChapterId, event);
  }, [projectRoot, currentChapterId, editor]);

  return {
    getCurrentChapterContent,
    getBlockById,
    getSelectedBlocks,
    getCurrentWordCount,
    insertBlocksAfter,
    replaceBlock,
    deleteBlocks,
    highlightBlocks,
    clearAiHighlights,
    recordHistoryEvent,
  };
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

async function calculateDocHash(doc: any): Promise<string> {
  const content = JSON.stringify(doc);
  const encoder = new TextEncoder();
  const data = encoder.encode(content);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map(b => b.toString(16).padStart(2, '0')).join('').slice(0, 16);
}
```

---

## 6. Patch 执行引擎

### 6.1 stores/editor-store.ts 中的 applyPatch

```typescript
applyPatch: async (patch: PatchOp[], proposalId?: string) => {
  const { editor, projectRoot, currentChapterId } = get();
  if (!editor || !projectRoot || !currentChapterId) return;

  const beforeDoc = editor.getJSON();
  const beforeHash = await calculateDocHash(beforeDoc);

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

  const afterDoc = editor.getJSON();
  const afterHash = await calculateDocHash(afterDoc);

  const event: ChapterHistoryEvent = {
    schema_version: 1,
    event_id: crypto.randomUUID(),
    created_at: Date.now(),
    actor: proposalId ? 'ai' : 'human',
    source_proposal_id: proposalId,
    before_hash: beforeHash,
    after_hash: afterHash,
    patch,
  };

  await commands.appendChapterHistoryEvent(projectRoot, currentChapterId, event);

  set({ isDirty: true });
},
```

---

## 7. AI 高亮颜色系统

### 7.1 颜色定义

```typescript
export const AI_HIGHLIGHT_COLORS = {
  'ai-new': '#bbf7d0',      // 绿色 - AI 新增内容
  'ai-modified': '#fef08a', // 黄色 - AI 修改内容
  'ai-pending': '#e9d5ff',  // 紫色 - AI 待确认内容
  'ai-deleted': '#fecaca',  // 红色 - AI 建议删除（显示为划线）
} as const;

export type AiHighlightColor = keyof typeof AI_HIGHLIGHT_COLORS;
```

### 7.2 CSS 样式

```css
.novel-highlight[data-color="ai-new"] {
  background-color: #bbf7d0;
}

.novel-highlight[data-color="ai-modified"] {
  background-color: #fef08a;
}

.novel-highlight[data-color="ai-pending"] {
  background-color: #e9d5ff;
}

.novel-highlight[data-color="ai-deleted"] {
  background-color: #fecaca;
  text-decoration: line-through;
}
```

---

## 8. Phase 2 预留接口

### 8.1 AI 服务接口（Phase 2 实现）

```typescript
interface AiService {
  generateContinuation: (params: {
    context: string;
    prompt: string;
    maxTokens?: number;
  }) => Promise<string>;

  generateVariations: (params: {
    content: string;
    count: number;
  }) => Promise<string[]>;

  improveWriting: (params: {
    content: string;
    instruction: string;
  }) => Promise<string>;

  convertToTiptapJson: (text: string) => TiptapBlock[];
}
```

### 8.2 右侧面板接口（Phase 2 实现）

```typescript
interface AiChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: number;
  proposalId?: string;
}

interface AiChatState {
  messages: AiChatMessage[];
  isGenerating: boolean;
  
  sendMessage: (content: string) => Promise<void>;
  acceptSuggestion: (proposalId: string) => Promise<void>;
  rejectSuggestion: (proposalId: string) => Promise<void>;
}
```

---

## 9. Phase 1 验收清单

### 9.1 数据存储
- [ ] `save_ai_proposal` 能保存 proposal JSON
- [ ] `append_chapter_history_event` 能追加 JSONL 事件
- [ ] 文件路径正确（`magic_novel/ai/proposals/`, `magic_novel/history/chapters/`）

### 9.2 前端能力
- [ ] `applyPatch` 能执行 insert_blocks
- [ ] `applyPatch` 能执行 update_block
- [ ] `applyPatch` 能执行 delete_blocks
- [ ] `highlightBlocks` 能高亮指定 block
- [ ] `clearAiHighlights` 能清除高亮

### 9.3 数据完整性
- [ ] History 事件包含 before_hash 和 after_hash
- [ ] Proposal 状态能正确更新
- [ ] Block ID 在操作后保持稳定

---

## 10. 测试用例

### 10.1 Patch 操作测试

```typescript
describe('Patch Operations', () => {
  it('should insert blocks after specified block', async () => {
    const { applyPatch } = useEditorStore.getState();
    
    const patch = [{
      op: 'insert_blocks',
      after_block_id: 'block-1',
      blocks: [{
        type: 'paragraph',
        attrs: { id: 'new-block' },
        content: [{ type: 'text', text: 'New content' }],
      }],
    }];

    await applyPatch(patch);
    
    const doc = editor.getJSON();
    const blockIds = doc.content.map((b: any) => b.attrs.id);
    expect(blockIds.indexOf('new-block')).toBe(blockIds.indexOf('block-1') + 1);
  });

  it('should update block content', async () => {
    const { applyPatch } = useEditorStore.getState();
    
    const patch = [{
      op: 'update_block',
      block_id: 'block-1',
      before: { type: 'paragraph', attrs: { id: 'block-1' }, content: [{ type: 'text', text: 'Old' }] },
      after: { type: 'paragraph', attrs: { id: 'block-1' }, content: [{ type: 'text', text: 'New' }] },
    }];

    await applyPatch(patch);
    
    const block = getBlockById('block-1');
    expect(block.content[0].text).toBe('New');
  });

  it('should delete blocks', async () => {
    const { applyPatch } = useEditorStore.getState();
    
    const patch = [{
      op: 'delete_blocks',
      block_ids: ['block-2', 'block-3'],
    }];

    await applyPatch(patch);
    
    const doc = editor.getJSON();
    const blockIds = doc.content.map((b: any) => b.attrs.id);
    expect(blockIds).not.toContain('block-2');
    expect(blockIds).not.toContain('block-3');
  });
});
```

---

## 11. 总结

Phase 1 AI 底座完成后，具备以下能力：

1. **存储层**：Proposal 和 History 的持久化
2. **执行层**：Patch 操作的前端执行引擎
3. **展示层**：AI 内容高亮系统
4. **追踪层**：变更历史记录

Phase 2 只需接入 LLM API 和实现 AI 聊天 UI，即可快速上线 AI 写作助手功能。

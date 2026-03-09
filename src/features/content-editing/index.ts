import {
  endWritingSession,
  getProjectTree,
  readChapter,
  saveChapter,
  saveChapterMarkdown,
  setChapterWordGoal,
  startWritingSession,
  updateWritingSession,
} from '@/lib/tauri-commands'
import { operationGetMarkdown } from '@/lib/operations'
import type { Editor } from '@tiptap/react'
import { mapBackendTreeNode } from '@/features/shared/tree-node-mapper'

export async function readChapterContent(projectPath: string, chapterPath: string) {
  return readChapter(projectPath, chapterPath)
}

export async function saveChapterContent(
  editor: Editor,
  projectPath: string,
  chapterPath: string,
) {
  const content = editor.getJSON()
  await saveChapter(projectPath, chapterPath, content)

  const markdown = operationGetMarkdown(editor)
  const mdPath = chapterPath.replace(/\.json$/, '.md')
  await saveChapterMarkdown(projectPath, mdPath, markdown)

  const tree = await getProjectTree(projectPath)
  return tree.map(mapBackendTreeNode)
}

export async function setChapterGoal(
  projectPath: string,
  chapterPath: string,
  wordGoal: number | null,
) {
  return setChapterWordGoal(projectPath, chapterPath, wordGoal)
}

export async function startWritingSessionFeature(
  projectPath: string,
  chapterPath: string | null,
  currentWordCount: number,
  rootDir?: string,
) {
  return startWritingSession(projectPath, chapterPath, currentWordCount, rootDir)
}

export async function updateWritingSessionFeature(
  currentWordCount: number,
  activeDurationSecs: number,
  idleDurationSecs: number,
  rootDir?: string,
) {
  await updateWritingSession(currentWordCount, activeDurationSecs, idleDurationSecs, rootDir)
}

export async function endWritingSessionFeature(finalWordCount: number, rootDir?: string) {
  await endWritingSession(finalWordCount, rootDir)
}

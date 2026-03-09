import { useMemo } from 'react'

interface WordCountResult {
  chars: number
  charsNoSpace: number
  words: number
  paragraphs: number
  readingTime: number // in minutes
}

export function useWordCount(content: unknown): WordCountResult {
  return useMemo(() => {
    if (!content) return { chars: 0, charsNoSpace: 0, words: 0, paragraphs: 0, readingTime: 0 }

    const text = extractText(content)
    const chars = text.length
    const charsNoSpace = text.replace(/\s/g, '').length
    const words = countWords(text)
    const paragraphs = countParagraphs(content)
    const readingTime = Math.ceil(words / 350) // 中文阅读速度约350字/分钟

    return { chars, charsNoSpace, words, paragraphs, readingTime }
  }, [content])
}

function extractText(value: unknown): string {
  if (typeof value === 'string') return value
  if (!value || typeof value !== 'object') return ''
  
  const obj = value as Record<string, unknown>
  
  if ('text' in obj && typeof obj.text === 'string') {
    return obj.text
  }
  
  if ('content' in obj && Array.isArray(obj.content)) {
    return obj.content.map(extractText).join('')
  }
  
  if (Array.isArray(value)) {
    return value.map(extractText).join('')
  }
  
  return ''
}

function countWords(text: string): number {
  let count = 0
  let inWord = false
  
  for (const char of text) {
    if (/\s/.test(char)) {
      if (inWord) {
        count++
        inWord = false
      }
    } else if (isCJK(char)) {
      if (inWord) {
        count++
        inWord = false
      }
      count++
    } else {
      inWord = true
    }
  }
  
  if (inWord) count++
  
  return count
}

function isCJK(char: string): boolean {
  const code = char.charCodeAt(0)
  return (
    (code >= 0x4e00 && code <= 0x9fff) ||
    (code >= 0x3400 && code <= 0x4dbf) ||
    (code >= 0x3000 && code <= 0x303f) ||
    (code >= 0x3040 && code <= 0x309f) ||
    (code >= 0x30a0 && code <= 0x30ff) ||
    (code >= 0xff00 && code <= 0xffef)
  )
}

function countParagraphs(content: unknown): number {
  if (!content || typeof content !== 'object') return 0

  let count = 0

  const traverse = (node: unknown) => {
    if (!node || typeof node !== 'object') return

    const maybeNode = node as { type?: string; content?: unknown[] }

    // 统计段落节点
    if (maybeNode.type === 'paragraph') {
      count++
    }

    // 递归遍历子节点
    if (Array.isArray(maybeNode.content)) {
      maybeNode.content.forEach(traverse)
    }
  }

  traverse(content)
  return count
}

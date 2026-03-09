import type { GlobalSearchResult } from '@/features/search-retrieval'

import type { SearchResultGroup } from './global-search-types'

export function escapeRegExp(input: string) {
 return input.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

export function highlightMatch(text: string, query: string) {
 const trimmed = query.trim()
 if (!trimmed) return text

 const safe = escapeRegExp(trimmed)
 const parts = text.split(new RegExp(`(${safe})`, 'gi'))

 return (
 <span>
 {parts.map((part, index) =>
 part.toLowerCase() === trimmed.toLowerCase() ? (
 <mark key={index} className="tag-yellow-bg dark:tag-yellow-bg ">
 {part}
 </mark>
 ) : (
 part
 ),
 )}
 </span>
 )
}

export function mapVolumeTitles(tree: Array<{ kind: string; path: string; name: string }>) {
 const map = new Map<string, string>()
 for (const node of tree) {
 if (node.kind === 'dir') {
 map.set(node.path, node.name)
 }
 }
 return map
}

export function toSearchGroups(
 hits: GlobalSearchResult[],
 volumeTitles: Map<string, string>,
): SearchResultGroup[] {
 const grouped = new Map<string, GlobalSearchResult[]>()

 for (const hit of hits) {
 const arr = grouped.get(hit.path) || []
 arr.push(hit)
 grouped.set(hit.path, arr)
 }

 const out: SearchResultGroup[] = []

 for (const [path, groupHits] of grouped.entries()) {
 groupHits.sort((a, b) => b.score - a.score)
 const top = groupHits[0]
 const volumeId = path.startsWith('.magic_novel/') ? '' : String(path).split('/')[0] || ''

 const extraSnippets = groupHits
 .slice(1)
 .map((item) => item.snippet)
 .filter((snippet, index, arr) => Boolean(snippet.trim()) && arr.indexOf(snippet) === index)
 .slice(0, 3)

 out.push({
 path,
 title: top.chapterTitle || volumeId || path,
 volumeTitle: volumeId ? volumeTitles.get(volumeId) : undefined,
 mainSnippet: top.snippet,
 extraSnippets,
 score: top.score,
 })
 }

 out.sort((a, b) => b.score - a.score)
 return out
}

export function mapDegradedReason(reason?: string) {
 if (!reason) return '检索能力受限'
 if (reason === 'vectors_unavailable') return '向量索引不可用，已降级为关键词搜索'
 if (reason === 'embedding_unavailable') return 'Embedding 不可用，已降级为关键词搜索'
 if (reason === 'embedding_disabled') return 'Embedding 开关已关闭，已降级为关键词搜索'
 if (reason === 'embedding_model_unavailable') return '未检测到 Embedding 模型，已降级为关键词搜索'
 if (reason === 'hybrid_not_ready') return 'Hybrid 尚未就绪，已降级为关键词搜索'
 return reason
}

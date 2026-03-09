import { FileText } from 'lucide-react'

import type { SearchResultGroup } from './global-search-types'
import { highlightMatch } from './global-search-utils'

export function SearchResultList(input: {
 query: string
 groups: SearchResultGroup[]
 onClose: () => void
 onResultClick: (chapterPath: string) => void
}) {
 return (
 <div className="divide-y divide-[var(--border-color)]">
 {input.groups.map((group) => (
 <div key={group.path} className="p-3 hover:opacity-90 transition-colors">
 <button
 onClick={() => {
 input.onResultClick(group.path)
 input.onClose()
 }}
 className="w-full text-left"
 >
 <div className="flex items-center gap-2 mb-1">
 <FileText className="h-4 w-4 text-muted-foreground" />
 <span className="text-sm font-medium">{group.title}</span>
 {group.volumeTitle ? <span className="text-xs text-muted-foreground">· {group.volumeTitle}</span> : null}
 </div>
 <div className="text-sm line-clamp-2">{highlightMatch(group.mainSnippet, input.query)}</div>
 </button>
 {group.extraSnippets.length > 0 ? (
 <details className="mt-2 pl-6">
 <summary className="text-xs cursor-pointer">更多片段（{group.extraSnippets.length}）</summary>
 <div className="mt-1 space-y-1">
 {group.extraSnippets.map((snippet, index) => (
 <div key={`${group.path}-extra-${index}`} className="text-xs line-clamp-2">
 {highlightMatch(snippet, input.query)}
 </div>
 ))}
 </div>
 </details>
 ) : null}
 </div>
 ))}
 </div>
 )
}

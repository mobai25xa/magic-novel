import { Search, X } from 'lucide-react'

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/magic-ui/components'

import type { SearchMode } from '@/features/search-retrieval'

import type { SearchHeaderProps } from './global-search-types'
import { mapDegradedReason } from './global-search-utils'

export function SearchHeader(input: SearchHeaderProps) {
 return (
 <div className="border-b">
 <div className="flex items-center justify-between p-3">
 <div className="flex items-center gap-2">
 <Search className="h-4 w-4 " />
 <span className="text-sm font-medium">搜索结果 {input.count > 0 && `(${input.count})`}</span>
 </div>
 <div className="flex items-center gap-2">
 <Select value={input.mode} onValueChange={(value) => input.onModeChange(value as SearchMode)}>
 <SelectTrigger size="sm" className="w-[132px] text-xs">
 <SelectValue />
 </SelectTrigger>
 <SelectContent>
 <SelectItem value="keyword">Keyword</SelectItem>
 <SelectItem value="hybrid">Hybrid</SelectItem>
 <SelectItem value="semantic">Semantic</SelectItem>
 </SelectContent>
 </Select>
 <button onClick={input.onClose} className="h-6 w-6 flex items-center justify-center rounded hover:opacity-80">
 <X className="h-4 w-4" />
 </button>
 </div>
 </div>
 {input.degraded ? (
 <div className="px-3 pb-2 text-xs text-warning">已降级：{mapDegradedReason(input.degradedReason)}</div>
 ) : null}
 </div>
 )
}

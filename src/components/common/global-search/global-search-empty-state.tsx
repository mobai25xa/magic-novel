import { Search } from 'lucide-react'
import { Spinner } from '@/magic-ui/components'

export function SearchEmptyState(input: {
 query: string
 isSearching: boolean
 error: string | null
}) {
 if (input.isSearching) {
 return (
 <div className="p-8 text-center ">
 <Spinner size="sm" className="mx-auto mb-2" />
 <div className="text-sm">搜索中...</div>
 </div>
 )
 }

 if (input.error) {
 return (
 <div className="p-8 text-center ">
 <div className="text-sm">搜索失败：{input.error}</div>
 </div>
 )
 }

 return (
 <div className="p-8 text-center ">
 <Search className="h-12 w-12 mx-auto mb-2 opacity-50" />
 <div className="text-sm">{input.query.trim() ? '未找到匹配结果' : '输入关键词开始搜索'}</div>
 </div>
 )
}

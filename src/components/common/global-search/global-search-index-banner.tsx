import type { SearchIndexBannerProps } from './global-search-types'

export function SearchIndexBanner(input: SearchIndexBannerProps) {
 if (input.mode === 'keyword') return null

 const state = input.status?.state || 'unknown'

 if (state === 'ready') return null

 if (state === 'building') {
 const done = input.status?.progress?.done ?? 0
 const total = input.status?.progress?.total ?? 0
 const stage = input.status?.progress?.stage || 'building'
 return (
 <div className="px-3 py-2 border-b text-xs flex items-center justify-between gap-2">
 <span>索引构建中：{stage} ({done}/{total || '?'})</span>
 <button
 type="button"
 disabled={input.loading}
 onClick={input.onCancel}
 className="px-2 py-1 rounded border hover:opacity-80 disabled:opacity-60"
 >
 取消
 </button>
 </div>
 )
 }

 if (state === 'failed') {
 return (
 <div className="px-3 py-2 border-b text-xs flex items-center justify-between gap-2">
 <span>索引构建失败：{input.status?.last_error || '请检查 embedding 与索引配置'}</span>
 <button
 type="button"
 disabled={input.loading}
 onClick={input.onBuild}
 className="px-2 py-1 rounded border hover:opacity-80 disabled:opacity-60"
 >
 重试构建
 </button>
 </div>
 )
 }

 return (
 <div className="px-3 py-2 border-b text-xs flex items-center justify-between gap-2">
 <span>Semantic / Hybrid 需要向量索引与 Embedding 配置</span>
 <button
 type="button"
 disabled={input.loading}
 onClick={input.onBuild}
 className="px-2 py-1 rounded border hover:opacity-80 disabled:opacity-60"
 >
 构建索引
 </button>
 </div>
 )
}

import { ArrowLeft } from 'lucide-react'
import { AssetManager } from '@/components/assets/AssetManager'

interface AssetsPageProps {
  onBack: () => void
}

export function AssetsPage({ onBack }: AssetsPageProps) {
  return (
    <div className="flex-1 overflow-hidden flex flex-col bg-background">
      <div className="page-header px-3 gap-2">
        <button
          onClick={onBack}
          className="toolbar-btn"
          aria-label="返回"
        >
          <ArrowLeft className="h-4 w-4" />
        </button>
        <h2 className="text-sm font-medium">资产管理</h2>
      </div>

      <div className="flex-1 min-h-0">
        <AssetManager />
      </div>
    </div>
  )
}

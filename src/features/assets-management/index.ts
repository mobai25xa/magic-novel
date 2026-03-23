import {
  createAssetFile,
  createAssetFolder,
  deleteAssetPath,
  getAssetsTree,
  listAssets,
  readAsset,
  readAssetFile,
  saveAsset,
  saveAssetFile,
  updateAssetFileTitle,
  updateAssetFolderTitle,
  type AssetKind,
  type AssetLibraryNode,
  type AssetSummary,
} from '@/lib/tauri-commands'

export type { AssetKind, AssetLibraryNode, AssetSummary }

export {
  listAssets,
  readAsset,
  saveAsset,
  readAssetFile,
  saveAssetFile,
  getAssetsTree,
  createAssetFolder,
  createAssetFile,
  updateAssetFileTitle,
  updateAssetFolderTitle,
  deleteAssetPath,
}

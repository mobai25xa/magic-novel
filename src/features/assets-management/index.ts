import {
  createMagicAssetFile,
  createMagicAssetFolder,
  deleteMagicAssetPath,
  getMagicAssetsTree,
  listAssets,
  readAsset,
  readMagicAsset,
  saveAsset,
  saveMagicAsset,
  updateMagicAssetFolderTitle,
  updateMagicAssetTitle,
  type AssetKind,
  type AssetSummary,
  type MagicAssetNode,
} from '@/lib/tauri-commands'

export type { AssetKind, AssetSummary, MagicAssetNode }

export {
  listAssets,
  readAsset,
  saveAsset,
  readMagicAsset,
  saveMagicAsset,
  getMagicAssetsTree,
  createMagicAssetFolder,
  createMagicAssetFile,
  updateMagicAssetTitle,
  updateMagicAssetFolderTitle,
  deleteMagicAssetPath,
}

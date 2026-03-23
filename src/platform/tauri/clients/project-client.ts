import { invokeTauri } from './core'

export type ProjectType = string

export interface ProjectMetadata {
  schema_version: number
  project_id: string
  name: string
  author: string
  description?: string
  cover_image?: string
  project_type: ProjectType[]
  target_total_words?: number
  planned_volumes?: number
  target_words_per_volume?: number
  target_words_per_chapter?: number
  narrative_pov?: string
  tone?: string[]
  audience?: string
  bootstrap_state?: string
  bootstrap_updated_at?: number
  created_at: number
  updated_at: number
  app_min_version?: string
  last_opened_at?: number
}

export interface FileNode {
  kind: 'dir' | 'chapter' | 'knowledge' | 'asset_dir' | 'asset_file'
  name: string
  path: string
  children?: FileNode[]
  chapter_id?: string
  title?: string
  text_length_no_whitespace?: number
  word_count?: number
  status?: string
  updated_at?: number
  created_at?: number
  assetRelativePath?: string
}

export interface ProjectSnapshot {
  project: ProjectMetadata
  path: string
  tree: FileNode[]
}

export interface VolumeMetadata {
  schema_version: number
  volume_id: string
  title: string
  summary?: string
  target_words?: number
  dramatic_goal?: string
  status?: string
  created_at: number
  updated_at: number
}

export interface Chapter {
  schema_version: number
  id: string
  title: string
  content: unknown
  counts: {
    text_length_no_whitespace: number
    word_count?: number
    algorithm_version: number
    last_calculated_at: number
  }
  target_words?: number
  status?: string
  summary?: string
  plot_goal?: string
  emotional_goal?: string
  tags?: string[]
  pinned_assets?: { kind: string; asset_id: string; node_ids?: string[] }[]
  last_cursor_position?: number
  created_at: number
  updated_at: number
}

export type ProjectBootstrapPhase =
  | 'pending'
  | 'assembling_prompt'
  | 'llm_generating'
  | 'writing_artifacts'
  | 'partially_generated'
  | 'ready_for_review'
  | 'ready_to_write'
  | 'failed'

export type ProjectBootstrapArtifactStatus = 'draft' | 'proposed' | 'accepted' | 'failed' | string

export interface ProjectBootstrapArtifact {
  kind: string
  path: string
  status: ProjectBootstrapArtifactStatus
  title?: string
  summary?: string
  updated_at?: number
}

export interface ProjectBootstrapStatus {
  project_id: string
  creation_job_id: string
  phase: ProjectBootstrapPhase
  progress: number
  bootstrap_state: string
  completed_steps: string[]
  failed_steps: string[]
  generated_artifacts: ProjectBootstrapArtifact[]
  recommended_next_action?: string
  error_message?: string
  started_at?: number
  updated_at?: number
}

export interface CreateProjectInput {
  path: string
  name: string
  author: string
  description?: string
  coverImage?: string
  projectType?: string[]
  targetTotalWords?: number
  plannedVolumes?: number
  targetWordsPerVolume?: number
  targetWordsPerChapter?: number
  narrativePov?: string
  tone?: string[]
  audience?: string
}

export interface StartProjectBootstrapInput {
  project_path: string
  creation_brief: string
  description?: string
  target_total_words?: number
  planned_volumes?: number
  target_words_per_volume?: number
  target_words_per_chapter?: number
  narrative_pov?: string
  tone?: string[]
  audience?: string
  protagonist_seed?: string
  counterpart_seed?: string
  world_seed?: string
  ending_direction?: string
}

export type RecycleItemType = 'novel' | 'chapter' | 'volume'

export interface RecycleItem {
  id: string
  type: RecycleItemType
  name: string
  origin: string
  description: string
  deleted_at: number
  days_remaining: number
}

type ChapterMetadataInput = {
  title?: string
  summary?: string
  status?: string
  targetWords?: number
  tags?: string[]
  pinnedAssets?: { kind: string; assetId: string; nodeIds?: string[] }[] | null
}

export async function createProjectClient(input: CreateProjectInput): Promise<ProjectSnapshot> {
  return invokeTauri('create_project', { ...input })
}

export async function openProjectClient(path: string): Promise<ProjectSnapshot> {
  return invokeTauri('open_project', { path })
}

export async function getProjectTreeClient(path: string): Promise<FileNode[]> {
  return invokeTauri('get_project_tree', { path })
}

export async function updateProjectMetadataClient(
  path: string,
  name?: string,
  author?: string,
  description?: string,
  projectType?: string[],
  coverImage?: string,
): Promise<ProjectMetadata> {
  return invokeTauri('update_project_metadata', { path, name, author, description, projectType, coverImage })
}

export async function scanProjectsDirectoryClient(rootDir: string): Promise<ProjectSnapshot[]> {
  return invokeTauri('scan_projects_directory', { rootDir })
}

export async function trashProjectClient(projectPath: string): Promise<void> {
  return invokeTauri('trash_project', { projectPath })
}

export async function listRecycledProjectsClient(rootDir: string): Promise<RecycleItem[]> {
  return invokeTauri('list_recycled_projects', { rootDir })
}

export async function restoreRecycledProjectClient(rootDir: string, itemId: string): Promise<void> {
  return invokeTauri('restore_recycled_project', { rootDir, itemId })
}

export async function permanentlyDeleteRecycledProjectClient(rootDir: string, itemId: string): Promise<void> {
  return invokeTauri('permanently_delete_recycled_project', { rootDir, itemId })
}

export async function emptyRecycledProjectsClient(rootDir: string): Promise<void> {
  return invokeTauri('empty_recycled_projects', { rootDir })
}

export async function clearWritingStatsClient(rootDir?: string): Promise<void> {
  return invokeTauri('clear_writing_stats', { rootDir })
}

export async function createVolumeClient(projectPath: string, title: string): Promise<VolumeMetadata> {
  return invokeTauri('create_volume', { projectPath, title })
}

export async function readVolumeClient(projectPath: string, volumePath: string): Promise<VolumeMetadata> {
  return invokeTauri('read_volume', { projectPath, volumePath })
}

export async function updateVolumeClient(
  projectPath: string,
  volumePath: string,
  title?: string,
  summary?: string,
): Promise<VolumeMetadata> {
  return invokeTauri('update_volume', { projectPath, volumePath, title, summary })
}

export async function trashVolumeClient(projectPath: string, volumePath: string): Promise<void> {
  return invokeTauri('trash_volume', { projectPath, volumePath })
}

export async function listRecycleItemsClient(projectPath: string): Promise<RecycleItem[]> {
  return invokeTauri('list_recycle_items', { projectPath })
}

export async function restoreRecycleItemClient(projectPath: string, itemId: string): Promise<void> {
  return invokeTauri('restore_recycle_item', { projectPath, itemId })
}

export async function permanentlyDeleteRecycleItemClient(projectPath: string, itemId: string): Promise<void> {
  return invokeTauri('permanently_delete_recycle_item', { projectPath, itemId })
}

export async function emptyRecycleBinClient(projectPath: string): Promise<void> {
  return invokeTauri('empty_recycle_bin', { projectPath })
}

export async function createChapterClient(
  projectPath: string,
  volumePath: string,
  title: string,
): Promise<Chapter> {
  return invokeTauri('create_chapter', { projectPath, volumePath, title })
}

export async function readChapterClient(projectPath: string, chapterPath: string): Promise<Chapter> {
  return invokeTauri('read_chapter', { projectPath, chapterPath })
}

export async function saveChapterClient(
  projectPath: string,
  chapterPath: string,
  content: unknown,
  title?: string,
): Promise<Chapter> {
  return invokeTauri('save_chapter', { projectPath, chapterPath, content, title })
}

export async function saveChapterMarkdownClient(
  projectPath: string,
  markdownPath: string,
  content: string,
): Promise<void> {
  return invokeTauri('save_chapter_markdown', { projectPath, markdownPath, content })
}

export async function updateChapterMetadataClient(
  projectPath: string,
  chapterPath: string,
  options: ChapterMetadataInput,
): Promise<Chapter> {
  return invokeTauri('update_chapter_metadata', {
    projectPath,
    chapterPath,
    title: options.title,
    summary: options.summary,
    status: options.status,
    targetWords: options.targetWords,
    tags: options.tags,
    pinnedAssets: options.pinnedAssets?.map((asset) => ({
      kind: asset.kind,
      assetId: asset.assetId,
      nodeIds: asset.nodeIds,
    })),
  })
}

export async function setChapterWordGoalClient(
  projectPath: string,
  chapterPath: string,
  wordGoal: number | null,
): Promise<Chapter> {
  return invokeTauri('set_chapter_word_goal', { projectPath, chapterPath, wordGoal })
}

export async function trashChapterClient(projectPath: string, chapterPath: string): Promise<void> {
  return invokeTauri('trash_chapter', { projectPath, chapterPath })
}

export async function moveChapterClient(
  projectPath: string,
  chapterPath: string,
  targetVolumePath: string,
  targetIndex: number,
): Promise<string> {
  return invokeTauri('move_chapter', { projectPath, chapterPath, targetVolumePath, targetIndex })
}

export async function importAssetClient(projectPath: string, inputPath: string, kind: string): Promise<string> {
  return invokeTauri('import_asset', { projectPath, inputPath, kind })
}

export async function importManuscriptClient(projectPath: string, inputPath: string): Promise<void> {
  return invokeTauri('import_manuscript', { projectPath, inputPath })
}

export async function importManuscriptIntoVolumeClient(
  projectPath: string,
  volumePath: string,
  inputPath: string,
): Promise<void> {
  return invokeTauri('import_manuscript_into_volume', { projectPath, volumePath, inputPath })
}

export async function importChapterClient(
  projectPath: string,
  volumePath: string,
  inputPath: string,
  title?: string,
): Promise<string> {
  return invokeTauri('import_chapter', { projectPath, volumePath, inputPath, title })
}

export async function exportChapterClient(
  projectPath: string,
  chapterPath: string,
  outputPath: string,
  format: string,
): Promise<void> {
  return invokeTauri('export_chapter', { projectPath, chapterPath, outputPath, format })
}

export async function exportVolumeClient(
  projectPath: string,
  volumePath: string,
  outputPath: string,
  format: string,
): Promise<void> {
  return invokeTauri('export_volume', { projectPath, volumePath, outputPath, format })
}

export async function exportBookSingleClient(
  projectPath: string,
  outputPath: string,
  format: string,
): Promise<void> {
  return invokeTauri('export_book_single', { projectPath, outputPath, format })
}

export async function exportTreeMultiClient(
  projectPath: string,
  outputDir: string,
  format: string,
): Promise<number> {
  return invokeTauri('export_tree_multi', { projectPath, outputDir, format })
}

export async function startProjectBootstrapClient(
  input: StartProjectBootstrapInput,
): Promise<ProjectBootstrapStatus> {
  return invokeTauri('start_project_bootstrap', { input })
}

export async function getProjectBootstrapStatusClient(projectPath: string): Promise<ProjectBootstrapStatus> {
  return invokeTauri('get_project_bootstrap_status', { projectPath })
}

export async function resumeProjectBootstrapClient(projectPath: string): Promise<ProjectBootstrapStatus> {
  return invokeTauri('resume_project_bootstrap', { projectPath })
}

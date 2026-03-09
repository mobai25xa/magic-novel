/** 全局页面枚举 — 对应 sidebar 导航项 */
export type AppPage =
  | 'workspace'    // 首页工作台
  | 'dashboard'    // 数据仪表盘
  | 'skills'       // 技能工坊
  | 'workers'      // AI Workers
  | 'recycle'      // 回收站
  | 'create'       // 创建小说
  | 'editor'       // 编辑器 — 需要 projectPath
  | 'settings'     // 设置 — 全页面

/** 设置子页面枚举 */
export type SettingsSubPage =
  | 'about'
  | 'general'
  | 'providers'
  | 'editor'
  | 'projects'
  | 'ai'

/** 导航状态 — navigation-store 的核心 state */
export interface NavigationState {
  currentPage: AppPage
  previousPage: AppPage | null
  settingsSubPage: SettingsSubPage
  sidebarCollapsed: boolean
}

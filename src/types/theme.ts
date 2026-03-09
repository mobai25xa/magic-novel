/**
 * HSL 颜色值
 */
export type HSLValue = [number, number, number]

/**
 * 主题颜色集合
 */
export interface ThemeColorSet {
  background: HSLValue | null
  foreground: HSLValue | null
  card: HSLValue | null
  cardForeground: HSLValue | null
  primary: HSLValue | null
  primaryForeground: HSLValue | null
  secondary: HSLValue | null
  secondaryForeground: HSLValue | null
  muted: HSLValue | null
  mutedForeground: HSLValue | null
  accent: HSLValue | null
  accentForeground: HSLValue | null
  border: HSLValue | null
  input: HSLValue | null
  ring: HSLValue | null
}

/**
 * 自定义主题颜色配置
 * 使用 HSL 格式，值为 [hue, saturation, lightness] 数组或 null
 * null 表示使用默认值
 */
export interface CustomThemeColors {
  // 亮色主题颜色
  light: ThemeColorSet
  // 暗色主题颜色
  dark: ThemeColorSet
}

/**
 * 主题模式
 */
export type ThemeMode = 'light' | 'dark' | 'system'

/**
 * 语言选项
 */
export type Language = 'zh' | 'en'

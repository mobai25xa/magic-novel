import { forwardRef, useMemo } from 'react'
import { BookOpen, FolderOpen } from 'lucide-react'
import { Filespan as MagicFilespan } from '@/magic-ui/components'

type FilespanProps = {
  path: string
  onClick?: (path: string) => void
  className?: string
}

const iconMap = {
  book: <BookOpen className="h-3 w-3 shrink-0" />,
  folder: <FolderOpen className="h-3 w-3 shrink-0" />,
} as const

function resolveIcon(path: string): keyof typeof iconMap {
  const normalized = path.replace(/\\/g, '/')
  const first = normalized.split('/').filter(Boolean)[0]
  if (first === 'assets') {
    return 'folder'
  }
  return 'book'
}

const Filespan = forwardRef<HTMLSpanElement, FilespanProps>(({ path, onClick, className }, ref) => {
  const iconKey = useMemo(() => resolveIcon(path), [path])

  return (
    <MagicFilespan
      ref={ref}
      path={path}
      icon={iconMap[iconKey]}
      className={className}
      copyable={Boolean(onClick)}
      onCopy={onClick ? () => onClick(path) : undefined}
    />
  )
})
Filespan.displayName = 'Filespan'

export { Filespan }
export type { FilespanProps }

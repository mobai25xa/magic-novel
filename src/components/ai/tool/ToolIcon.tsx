import {
  ArrowRightLeft,
  BookOpen,
  Eye,
  Trash2,
  Pencil,
  FolderTree,
  Search,
  ListTodo,
  Wrench,
} from 'lucide-react'

import type { ToolIconName } from './tool-icon-map'

const ICON_CLASS_NAME = 'h-3.5 w-3.5 shrink-0 '

export function ToolIcon({ name }: { name: ToolIconName }) {
  if (name === 'create') return <BookOpen className={ICON_CLASS_NAME} />
  if (name === 'read') return <Eye className={ICON_CLASS_NAME} />
  if (name === 'edit') return <Pencil className={ICON_CLASS_NAME} />
  if (name === 'delete') return <Trash2 className={ICON_CLASS_NAME} />
  if (name === 'move') return <ArrowRightLeft className={ICON_CLASS_NAME} />
  if (name === 'ls') return <FolderTree className={ICON_CLASS_NAME} />
  if (name === 'grep') return <Search className={ICON_CLASS_NAME} />
  if (name === 'todo') return <ListTodo className={ICON_CLASS_NAME} />
  return <Wrench className={ICON_CLASS_NAME} />
}

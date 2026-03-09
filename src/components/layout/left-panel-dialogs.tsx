import { InputDialog } from '@/components/common/InputDialog'
import { SelectDialog } from '@/components/common/SelectDialog'

type InputDialogState = {
  open: boolean
  title: string
  placeholder: string
  onConfirm: (value: string) => void
} | null

type SelectVolumeDialogState = {
  open: boolean
  chapterTitle: string
} | null

export function LeftPanelInputDialog(input: {
  state: InputDialogState
  onClose: () => void
}) {
  if (!input.state) return null

  return (
    <InputDialog
      open={input.state.open}
      title={input.state.title}
      placeholder={input.state.placeholder}
      onClose={input.onClose}
      onConfirm={input.state.onConfirm}
    />
  )
}

export function LeftPanelTocSortDialog(input: {
  open: boolean
  onClose: () => void
  onConfirm: (value: string) => void
}) {
  return (
    <SelectDialog
      open={input.open}
      title="排序"
      label="选择排序方式"
      options={[
        { value: 'manual:asc', label: '自定义顺序（可拖拽）' },
        { value: 'name:asc', label: '名称（升序）' },
        { value: 'name:desc', label: '名称（降序）' },
        { value: 'createdAt:asc', label: '创建时间（升序）' },
        { value: 'createdAt:desc', label: '创建时间（降序）' },
        { value: 'updatedAt:asc', label: '修改时间（升序）' },
        { value: 'updatedAt:desc', label: '修改时间（降序）' },
      ]}
      onClose={input.onClose}
      onConfirm={input.onConfirm}
    />
  )
}

export function LeftPanelPinnedAssetsDialog(input: {
  open: boolean
  options: { value: string; label: string }[]
  defaultValue?: string
  onClose: () => void
  onConfirm: (value: string) => void
}) {
  return (
    <SelectDialog
      open={input.open}
      title="绑定知识库"
      label="选择要绑定到当前章节的资产"
      options={input.options}
      defaultValue={input.defaultValue}
      onClose={input.onClose}
      onConfirm={input.onConfirm}
    />
  )
}

export function LeftPanelSelectVolumeDialog(input: {
  state: SelectVolumeDialogState
  options: { value: string; label: string }[]
  title: string
  label: string
  onClose: () => void
  onConfirm: (volumePath: string, chapterTitle: string) => void
}) {
  if (!input.state) return null

  return (
    <SelectDialog
      open={input.state.open}
      title={input.title}
      label={input.label}
      options={input.options}
      onClose={input.onClose}
      onConfirm={(volumePath) => input.onConfirm(volumePath, input.state!.chapterTitle)}
    />
  )
}

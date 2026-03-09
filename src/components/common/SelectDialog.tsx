import { useState } from 'react'
import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from '@/magic-ui/components'
import { Select, SelectTrigger, SelectValue, SelectContent, SelectItem } from '@/magic-ui/components'
import { Button } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

interface SelectOption {
 value: string
 label: string
}

interface SelectDialogProps {
 open: boolean
 title: string
 label?: string
 options: SelectOption[]
 defaultValue?: string
 closeOnConfirm?: boolean
 onClose: () => void
 onConfirm: (value: string) => void
}

export function SelectDialog({
 open,
 title,
 label,
 options,
 defaultValue,
 closeOnConfirm = true,
 onClose,
 onConfirm
}: SelectDialogProps) {
 const [value, setValue] = useState(defaultValue || options[0]?.value || '')
 const { translations } = useTranslation()
 const lb = label ?? translations.common.selectPlaceholder

 const handleConfirm = () => {
 if (value) {
 onConfirm(value)
 if (closeOnConfirm) {
 onClose()
 }
 }
 }

 return (
 <Modal
 open={open}
 onOpenChange={(isOpen) => {
 if (isOpen) {
 if (defaultValue) {
 setValue(defaultValue)
 } else if (options.length > 0) {
 setValue(options[0].value)
 }
 } else {
 onClose()
 }
 }}
 >
 <ModalContent size="sm">
 <ModalHeader>
 <ModalTitle>{title}</ModalTitle>
 <ModalDescription className="sr-only">{title}</ModalDescription>
 </ModalHeader>
 <div className="p-6">
 <label className="block text-sm font-medium mb-2">{lb}</label>

 <Select value={value} onValueChange={setValue}>
 <SelectTrigger className="w-full mb-6">
 <SelectValue />
 </SelectTrigger>
 <SelectContent>
 {options.map((option) => (
 <SelectItem key={option.value} value={option.value}>
 {option.label}
 </SelectItem>
 ))}
 </SelectContent>
 </Select>

 <div className="flex justify-end gap-3">
 <Button variant="secondary" onClick={onClose}>
 {translations.common.cancel}
 </Button>
 <Button
 variant="default"
 onClick={handleConfirm}
 disabled={!value}
 >
 {translations.common.confirm}
 </Button>
 </div>
 </div>
 </ModalContent>
 </Modal>
 )
}
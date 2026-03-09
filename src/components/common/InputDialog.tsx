import { useState } from 'react'
import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from '@/magic-ui/components'
import { Input } from '@/magic-ui/components'
import { Button } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

interface InputDialogProps {
 open: boolean
 title: string
 placeholder?: string
 defaultValue?: string
 onClose: () => void
 onConfirm: (value: string) => void
}

export function InputDialog({
 open,
 title,
 placeholder,
 defaultValue = '',
 onClose,
 onConfirm
}: InputDialogProps) {
 const [value, setValue] = useState(defaultValue)
 const { translations } = useTranslation()
 const ph = placeholder ?? translations.common.inputPlaceholder

 const handleConfirm = () => {
 const trimmedValue = value.trim()
 if (trimmedValue) {
 onConfirm(trimmedValue)
 onClose()
 }
 }

 const handleKeyPress = (e: React.KeyboardEvent) => {
 if (e.key === 'Enter') {
 handleConfirm()
 }
 }

 return (
 <Modal
 open={open}
 onOpenChange={(isOpen) => {
 if (isOpen) {
 setValue(defaultValue)
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
 <Input
 value={value}
 onChange={(e) => setValue(e.target.value)}
 onKeyDown={handleKeyPress}
 placeholder={ph}
 autoFocus
 className="mb-6"
 />

 <div className="flex justify-end gap-3">
 <Button variant="secondary" onClick={onClose}>
 {translations.common.cancel}
 </Button>
 <Button
 variant="default"
 onClick={handleConfirm}
 disabled={!value.trim()}
 >
 {translations.common.confirm}
 </Button>
 </div>
 </div>
 </ModalContent>
 </Modal>
 )
}
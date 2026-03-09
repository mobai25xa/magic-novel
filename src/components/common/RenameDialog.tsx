import { useState } from 'react'
import { Modal, ModalContent, ModalDescription, ModalHeader, ModalTitle } from '@/magic-ui/components'
import { Input } from '@/magic-ui/components'
import { Button } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

interface RenameDialogProps {
 open: boolean
 title: string
 defaultValue: string
 onClose: () => void
 onConfirm: (newName: string) => void
}

export function RenameDialog({ open, title, defaultValue, onClose, onConfirm }: RenameDialogProps) {
 const [name, setName] = useState(defaultValue)
 const { translations } = useTranslation()

 const handleConfirm = () => {
 if (name.trim() && name !== defaultValue) {
 onConfirm(name.trim())
 }
 onClose()
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
 setName(defaultValue)
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
 value={name}
 onChange={(e) => setName(e.target.value)}
 onKeyDown={handleKeyPress}
 placeholder={translations.common.inputPlaceholder}
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
 disabled={!name.trim() || name === defaultValue}
 >
 {translations.common.confirm}
 </Button>
 </div>
 </div>
 </ModalContent>
 </Modal>
 )
}
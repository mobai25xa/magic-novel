import React from 'react'
import {
 Modal,
 ModalContent,
 ModalFooter,
 ModalHeader,
 ModalTitle,
 ModalDescription,
} from '@/magic-ui/components'
import { Button } from '@/magic-ui/components'
import { useTranslation } from '@/hooks/use-translation'

interface ConfirmDialogProps {
 open: boolean
 title: string
 description: string
 confirmText?: string
 cancelText?: string
 danger?: boolean
 onConfirm: () => void
 onCancel: () => void
}

export function ConfirmDialog({
 open,
 title,
 description,
 confirmText,
 cancelText,
 danger = false,
 onConfirm,
 onCancel,
}: ConfirmDialogProps) {
 const { translations } = useTranslation()
 const ct = confirmText ?? translations.common.confirm
 const cc = cancelText ?? translations.common.cancel
 return (
 <Modal open={open} onOpenChange={(open) => !open && onCancel()}>
 <ModalContent size="sm" className="text-center">
 <ModalHeader>
 <ModalTitle>{title}</ModalTitle>
 <ModalDescription className="mt-2">{description}</ModalDescription>
 </ModalHeader>
 <ModalFooter className="justify-center gap-2 sm:gap-2">
 <Button
 onClick={onCancel}
 variant="secondary"
 >
 {cc}
 </Button>
 <Button
 onClick={onConfirm}
 variant={danger ? 'destructive' : 'default'}
 >
 {ct}
 </Button>
 </ModalFooter>
 </ModalContent>
 </Modal>
 )
}

'use client'

import type { FC, ReactNode } from 'react'
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogDescription,
    AlertDialogFooter,
    AlertDialogHeader,
    AlertDialogTitle,
} from '@shared/ui/alert-dialog'

type ConfirmActionProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    title: string
    description: ReactNode
    confirmLabel: string
    confirmDisabled?: boolean
    onConfirm: () => void
}

export const ConfirmAction: FC<ConfirmActionProps> = ({ open, onOpenChange, title, description, confirmLabel, confirmDisabled, onConfirm }) => (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
        <AlertDialogContent className='sm:max-w-md'>
            <AlertDialogHeader>
                <AlertDialogTitle>{title}</AlertDialogTitle>
                <AlertDialogDescription>{description}</AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
                <AlertDialogCancel variant='ghost'>취소</AlertDialogCancel>
                <AlertDialogAction variant='destructive' disabled={confirmDisabled} onClick={onConfirm}>
                    {confirmLabel}
                </AlertDialogAction>
            </AlertDialogFooter>
        </AlertDialogContent>
    </AlertDialog>
)

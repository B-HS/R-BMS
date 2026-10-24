'use client'

import { type FC, type ReactNode, type RefObject } from 'react'
import { useTranslations } from 'next-intl'
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
import { useDialogFocusReturn } from '@features/dialog/use-dialog-focus-return'

type ConfirmActionProps = {
    open: boolean
    onOpenChange: (open: boolean) => void
    title: string
    description: ReactNode
    confirmLabel: string
    confirmDisabled?: boolean
    onConfirm: () => void
    cancelLabel?: string
    returnFocusRef?: RefObject<HTMLElement | null>
}

export const ConfirmAction: FC<ConfirmActionProps> = ({
    open,
    onOpenChange,
    title,
    description,
    confirmLabel,
    confirmDisabled,
    onConfirm,
    cancelLabel,
    returnFocusRef,
}) => {
    const t = useTranslations('pages.shared')
    const focusReturn = useDialogFocusReturn(returnFocusRef)

    return (
        <AlertDialog open={open} onOpenChange={onOpenChange}>
            <AlertDialogContent className='sm:max-w-md' {...focusReturn}>
                <AlertDialogHeader>
                    <AlertDialogTitle>{title}</AlertDialogTitle>
                    <AlertDialogDescription>{description}</AlertDialogDescription>
                </AlertDialogHeader>
                <AlertDialogFooter>
                    <AlertDialogCancel variant='ghost'>{cancelLabel ?? t('cancel')}</AlertDialogCancel>
                    <AlertDialogAction variant='destructive' disabled={confirmDisabled} onClick={onConfirm}>
                        {confirmLabel}
                    </AlertDialogAction>
                </AlertDialogFooter>
            </AlertDialogContent>
        </AlertDialog>
    )
}

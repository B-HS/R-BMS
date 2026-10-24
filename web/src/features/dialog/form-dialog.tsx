'use client'

import { type FC, type PropsWithChildren, type ReactNode, type RefObject } from 'react'
import { useTranslations } from 'next-intl'
import { Button } from '@shared/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'
import { useDialogFocusReturn } from '@features/dialog/use-dialog-focus-return'

const SIZE_CLASS = { md: 'sm:max-w-md', lg: 'sm:max-w-2xl' }

type FormDialogProps = PropsWithChildren<{
    open: boolean
    onOpenChange: (open: boolean) => void
    title: string
    description?: ReactNode
    submitLabel: string
    submitDisabled?: boolean
    onSubmit: () => void
    size?: keyof typeof SIZE_CLASS
    returnFocusRef?: RefObject<HTMLElement | null>
}>

export const FormDialog: FC<FormDialogProps> = ({
    open,
    onOpenChange,
    title,
    description,
    submitLabel,
    submitDisabled,
    onSubmit,
    size = 'lg',
    returnFocusRef,
    children,
}) => {
    const t = useTranslations('pages.shared')
    const focusReturn = useDialogFocusReturn(returnFocusRef)

    return (
        <Dialog open={open} onOpenChange={onOpenChange}>
            <DialogContent className={SIZE_CLASS[size]} {...focusReturn}>
                <DialogHeader>
                    <DialogTitle>{title}</DialogTitle>
                    {description && <DialogDescription>{description}</DialogDescription>}
                </DialogHeader>
                <form
                    className='flex flex-col gap-4'
                    noValidate
                    onSubmit={(event) => {
                        event.preventDefault()
                        onSubmit()
                    }}>
                    {children}
                    <DialogFooter>
                        <Button type='button' variant='ghost' onClick={() => onOpenChange(false)}>
                            {t('cancel')}
                        </Button>
                        <Button type='submit' disabled={submitDisabled}>
                            {submitLabel}
                        </Button>
                    </DialogFooter>
                </form>
            </DialogContent>
        </Dialog>
    )
}

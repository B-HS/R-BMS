'use client'

import type { FC, PropsWithChildren, ReactNode } from 'react'
import { Button } from '@shared/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@shared/ui/dialog'

type FormDialogProps = PropsWithChildren<{
    open: boolean
    onOpenChange: (open: boolean) => void
    title: string
    description?: ReactNode
    submitLabel: string
    submitDisabled?: boolean
    onSubmit: () => void
}>

export const FormDialog: FC<FormDialogProps> = ({ open, onOpenChange, title, description, submitLabel, submitDisabled, onSubmit, children }) => (
    <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className='sm:max-w-2xl'>
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
                        취소
                    </Button>
                    <Button type='submit' disabled={submitDisabled}>
                        {submitLabel}
                    </Button>
                </DialogFooter>
            </form>
        </DialogContent>
    </Dialog>
)

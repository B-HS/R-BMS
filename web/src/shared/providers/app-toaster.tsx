'use client'

import type { ComponentProps, FC } from 'react'
import { useTheme } from 'next-themes'
import { Toaster } from 'sonner'

const TOASTER_CLASS_NAME =
    '[--border-radius:0px] [--normal-bg:var(--popover)] [--normal-text:var(--popover-foreground)] [--normal-border:var(--border)] [--normal-bg-hover:var(--popover)] [--normal-border-hover:var(--border)]'

const TOAST_OPTIONS = {
    classNames: {
        toast: 'rounded-none! border! border-border! text-sm! shadow-none!',
        success: 'border-l-4! border-l-primary!',
        error: 'border-l-4! border-l-destructive!',
        warning: 'border-l-4! border-l-warning!',
        info: 'border-l-4! border-l-ring!',
    },
} satisfies ComponentProps<typeof Toaster>['toastOptions']

export const AppToaster: FC = () => {
    const { resolvedTheme } = useTheme()

    return (
        <Toaster
            position='bottom-right'
            theme={resolvedTheme === 'light' ? 'light' : 'dark'}
            containerAriaLabel='알림'
            className={TOASTER_CLASS_NAME}
            toastOptions={TOAST_OPTIONS}
        />
    )
}

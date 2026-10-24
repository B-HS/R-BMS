'use client'

import { useEffect, type FC, type PropsWithChildren } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { useTranslations } from 'next-intl'
import { ThemeProvider } from 'next-themes'
import { TooltipProvider } from '@shared/ui/tooltip'
import { THEME_STORAGE_KEY } from '@shared/constants/theme'
import { AppToaster } from '@shared/providers/app-toaster'
import { bindMutationErrorMessages, getQueryClient } from '@shared/utils/get-query-client'

const TOOLTIP_DELAY_MS = 200

export const Providers: FC<PropsWithChildren> = ({ children }) => {
    const t = useTranslations('pages.errors')

    useEffect(() => bindMutationErrorMessages({ http: (status) => t('http', { status }), unknown: t('unknown') }), [t])

    return (
        <ThemeProvider
            attribute={['class', 'data-theme']}
            defaultTheme='dark'
            enableSystem={false}
            disableTransitionOnChange
            storageKey={THEME_STORAGE_KEY}>
            <QueryClientProvider client={getQueryClient()}>
                <TooltipProvider delayDuration={TOOLTIP_DELAY_MS}>
                    {children}
                    <AppToaster />
                </TooltipProvider>
            </QueryClientProvider>
        </ThemeProvider>
    )
}

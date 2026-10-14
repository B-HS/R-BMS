'use client'

import { type FC, type PropsWithChildren } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { ThemeProvider } from 'next-themes'
import { TooltipProvider } from '@shared/ui/tooltip'
import { THEME_STORAGE_KEY } from '@shared/constants/theme'
import { AppToaster } from '@shared/providers/app-toaster'
import { getQueryClient } from '@shared/utils/get-query-client'

const TOOLTIP_DELAY_MS = 200

export const Providers: FC<PropsWithChildren> = ({ children }) => (
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

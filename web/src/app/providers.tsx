'use client'

import { type FC, type PropsWithChildren } from 'react'
import { QueryClientProvider } from '@tanstack/react-query'
import { ThemeProvider } from 'next-themes'
import { Toaster } from '@shared/ui/sonner'
import { TooltipProvider } from '@shared/ui/tooltip'
import { THEME_STORAGE_KEY } from '@shared/constants/theme'
import { getQueryClient } from '@shared/utils/get-query-client'

export const Providers: FC<PropsWithChildren> = ({ children }) => (
    <ThemeProvider attribute={['class', 'data-theme']} defaultTheme='system' enableSystem storageKey={THEME_STORAGE_KEY}>
        <QueryClientProvider client={getQueryClient()}>
            <TooltipProvider delayDuration={200}>
                {children}
                <Toaster position='top-right' />
            </TooltipProvider>
        </QueryClientProvider>
    </ThemeProvider>
)

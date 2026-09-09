'use client'

import type { FC } from 'react'
import { Moon, Sun } from 'lucide-react'
import { useTheme } from 'next-themes'
import { Tooltip, TooltipContent, TooltipTrigger } from '@shared/ui/tooltip'

const TOGGLE_LABEL = '라이트 · 다크 테마 전환'

export const ThemeToggle: FC = () => {
    const { resolvedTheme, setTheme } = useTheme()

    return (
        <Tooltip>
            <TooltipTrigger
                aria-label={TOGGLE_LABEL}
                onClick={() => setTheme(resolvedTheme === 'dark' ? 'light' : 'dark')}
                className='text-sidebar-foreground hover:text-foreground inline-flex size-8 items-center justify-center'>
                <Moon className='size-4 dark:hidden' aria-hidden />
                <Sun className='hidden size-4 dark:block' aria-hidden />
            </TooltipTrigger>
            <TooltipContent>{TOGGLE_LABEL}</TooltipContent>
        </Tooltip>
    )
}

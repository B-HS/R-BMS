'use client'

import type { FC } from 'react'
import { Moon, Sun } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { useTheme } from 'next-themes'
import { FOOTER_ICON_BUTTON_CLASS_NAME, FOOTER_MENU_ITEM_CLASS_NAME } from '@shared/constants/shell-footer'
import { SidebarMenuButton, SidebarMenuItem, useSidebar } from '@shared/ui/sidebar'

export const ShellThemeControl: FC = () => {
    const { resolvedTheme, setTheme } = useTheme()
    const { isMobile, state } = useSidebar()
    const t = useTranslations('common.shell')

    return (
        <SidebarMenuItem className={FOOTER_MENU_ITEM_CLASS_NAME}>
            <SidebarMenuButton
                className={FOOTER_ICON_BUTTON_CLASS_NAME}
                aria-label={t('themeToggle')}
                tooltip={{ children: t('themeToggle'), hidden: isMobile, side: state === 'collapsed' ? 'right' : 'top' }}
                onClick={() => setTheme(resolvedTheme === 'light' ? 'dark' : 'light')}>
                <Sun className='hidden dark:block' />
                <Moon className='dark:hidden' />
            </SidebarMenuButton>
        </SidebarMenuItem>
    )
}

'use client'

import type { FC } from 'react'
import { useRouter } from 'next/navigation'
import { Languages } from 'lucide-react'
import { useLocale, useTranslations } from 'next-intl'
import { FOOTER_ICON_BUTTON_CLASS_NAME, FOOTER_MENU_ITEM_CLASS_NAME } from '@shared/constants/shell-footer'
import { buildLocaleCookie, isLocale, LOCALE_NATIVE_NAME, LOCALES } from '@shared/i18n/locale'
import { DropdownMenu, DropdownMenuContent, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger } from '@shared/ui/dropdown-menu'
import { SidebarMenuButton, SidebarMenuItem, useSidebar } from '@shared/ui/sidebar'

export const LocaleSwitcher: FC = () => {
    const router = useRouter()
    const locale = useLocale()
    const t = useTranslations('common.localeSwitcher')
    const { isMobile, state } = useSidebar()

    const handleLocaleChange = (value: string) => {
        if (!isLocale(value) || value === locale) return
        document.cookie = buildLocaleCookie(value)
        router.refresh()
    }

    return (
        <SidebarMenuItem className={FOOTER_MENU_ITEM_CLASS_NAME}>
            <DropdownMenu>
                <DropdownMenuTrigger asChild>
                    <SidebarMenuButton
                        className={FOOTER_ICON_BUTTON_CLASS_NAME}
                        aria-label={t('label')}
                        tooltip={{ children: t('label'), hidden: isMobile, side: state === 'collapsed' ? 'right' : 'top' }}>
                        <Languages />
                    </SidebarMenuButton>
                </DropdownMenuTrigger>
                <DropdownMenuContent align='start' side='top'>
                    <DropdownMenuRadioGroup value={locale} onValueChange={handleLocaleChange}>
                        {LOCALES.map((option) => (
                            <DropdownMenuRadioItem key={option} value={option} lang={option}>
                                {LOCALE_NATIVE_NAME[option]}
                            </DropdownMenuRadioItem>
                        ))}
                    </DropdownMenuRadioGroup>
                </DropdownMenuContent>
            </DropdownMenu>
        </SidebarMenuItem>
    )
}

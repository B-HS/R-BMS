'use client'

import { Suspense, useState, type CSSProperties, type FC, type PropsWithChildren } from 'react'
import Link from 'next/link'
import { useTranslations } from 'next-intl'
import { NAV_ITEMS } from '@shared/constants/navigation'
import { SITE_NAME } from '@shared/constants/site'
import {
    SHELL_FOOTER_COLLAPSED_HEIGHT_PX,
    SHELL_RAIL_CHROME_HEIGHT_PX,
    SHELL_SIDEBAR_COLLAPSED_WIDTH_PX,
    SHELL_SIDEBAR_MOBILE_WIDTH_PX,
    SHELL_SIDEBAR_WIDTH_PX,
} from '@shared/constants/ui'
import { ShellSidebarSlotContext } from '@shared/providers/shell-sidebar-slot-context'
import { ScrollIndicator } from '@shared/ui/scroll-indicator'
import { Sidebar, SidebarContent, SidebarFooter, SidebarHeader, SidebarInset, SidebarMenu, SidebarProvider, SidebarTrigger } from '@shared/ui/sidebar'
import { LocaleSwitcher } from '@widgets/locale-switcher/locale-switcher'
import { ShellAccountControl } from '@widgets/shell/shell-account-control'
import { ShellNavMenu } from '@widgets/shell/shell-nav-menu'
import { ShellThemeControl } from '@widgets/shell/shell-theme-control'

const SKIP_LINK_TARGET_ID = 'main-content'

export const AppShell: FC<PropsWithChildren> = ({ children }) => {
    const [sidebarSlotElement, setSidebarSlotElement] = useState<HTMLElement | null>(null)

    const t = useTranslations('common.shell')

    return (
        <SidebarProvider
            className='min-h-dvh w-full md:h-full md:min-h-0 md:overflow-hidden'
            style={
                {
                    '--sidebar-width': `${SHELL_SIDEBAR_WIDTH_PX}px`,
                    '--sidebar-width-icon': `${SHELL_SIDEBAR_COLLAPSED_WIDTH_PX}px`,
                    '--sidebar-width-mobile': `${SHELL_SIDEBAR_MOBILE_WIDTH_PX}px`,
                } as CSSProperties
            }>
            <a
                href={`#${SKIP_LINK_TARGET_ID}`}
                className='bg-primary text-primary-foreground sr-only z-(--z-overlay) px-3 py-2 text-sm font-medium focus:not-sr-only focus:fixed focus:top-2 focus:left-2'>
                {t('skipToContent')}
            </a>
            <ShellSidebarSlotContext.Provider value={{ element: sidebarSlotElement }}>
                <div className='flex min-h-dvh w-full md:h-full md:min-h-0 md:overflow-hidden'>
                    <Sidebar collapsible='icon' className='border-r-0 group-data-[side=left]:border-r-0'>
                        <SidebarHeader className='flex shrink-0 flex-row items-center gap-2 p-0' style={{ height: SHELL_RAIL_CHROME_HEIGHT_PX }}>
                            <SidebarTrigger aria-label={t('toggleSidebar')} className='ml-2 size-8 shrink-0' />
                            <Link
                                href='/'
                                aria-label={t('homeLink', { siteName: SITE_NAME })}
                                className='text-foreground flex min-w-0 items-center gap-2 group-data-[collapsible=icon]:hidden'>
                                <span className='truncate text-sm font-semibold tracking-tight'>{SITE_NAME}</span>
                            </Link>
                        </SidebarHeader>
                        <SidebarContent className='min-h-0 p-0'>
                            <nav aria-label={t('mainNav')} className='min-w-0'>
                                <ShellNavMenu items={NAV_ITEMS} />
                            </nav>
                            <div
                                ref={setSidebarSlotElement}
                                className='border-border min-w-0 border-y group-data-[collapsible=icon]:hidden empty:hidden'
                            />
                        </SidebarContent>
                        <SidebarFooter
                            className='bg-sidebar h-(--shell-footer-row-height) shrink-0 gap-0 p-0 group-data-[collapsible=icon]:h-(--shell-footer-collapsed-height)'
                            style={
                                {
                                    '--shell-footer-row-height': `${SHELL_RAIL_CHROME_HEIGHT_PX}px`,
                                    '--shell-footer-collapsed-height': `${SHELL_FOOTER_COLLAPSED_HEIGHT_PX}px`,
                                } as CSSProperties
                            }>
                            <SidebarMenu className='grid h-full grid-cols-[auto_auto_minmax(0,1fr)] items-stretch gap-0 group-data-[collapsible=icon]:auto-rows-fr group-data-[collapsible=icon]:grid-cols-1'>
                                <ShellThemeControl />
                                <LocaleSwitcher />
                                <Suspense fallback={null}>
                                    <ShellAccountControl />
                                </Suspense>
                            </SidebarMenu>
                        </SidebarFooter>
                    </Sidebar>
                    <SidebarInset
                        id={SKIP_LINK_TARGET_ID}
                        className='min-h-dvh min-w-0 rounded-none shadow-none md:h-full md:min-h-0 md:overflow-hidden'>
                        <div className='bg-background flex min-h-dvh min-w-0 flex-1 flex-col md:h-full md:min-h-0 md:overflow-hidden'>{children}</div>
                    </SidebarInset>
                </div>
            </ShellSidebarSlotContext.Provider>
            <ScrollIndicator className='md:hidden' />
        </SidebarProvider>
    )
}

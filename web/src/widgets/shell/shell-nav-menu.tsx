'use client'

import { Suspense, type FC } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useTranslations } from 'next-intl'
import type { NavItem } from '@shared/constants/navigation'
import { SidebarMenu, SidebarMenuButton, SidebarMenuItem, useSidebar } from '@shared/ui/sidebar'

type ShellNavMenuProps = {
    items: readonly NavItem[]
}

const NAV_MENU_BUTTON_CLASS_NAME =
    'h-9 rounded-none px-3 group-data-[collapsible=icon]:h-9! group-data-[collapsible=icon]:w-full! group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:p-0!'

const isActivePath = (pathname: string, href: string) => (href === '/' ? pathname === href : pathname === href || pathname.startsWith(`${href}/`))

const ShellNavMenuItems: FC<ShellNavMenuProps & { pathname: string | null }> = ({ items, pathname }) => {
    const t = useTranslations('common.nav')
    const { isMobile, setOpenMobile } = useSidebar()

    return (
        <SidebarMenu className='gap-0 p-0'>
            {items.map(({ href, labelKey, icon: Icon }) => {
                const text = t(labelKey)
                return (
                    <SidebarMenuItem key={href} className='min-w-0 flex-1'>
                        <SidebarMenuButton
                            asChild
                            isActive={pathname !== null && isActivePath(pathname, href)}
                            tooltip={text}
                            className={NAV_MENU_BUTTON_CLASS_NAME}>
                            <Link
                                href={href}
                                aria-label={text}
                                aria-current={pathname !== null && isActivePath(pathname, href) ? 'page' : undefined}
                                onClick={() => isMobile && setOpenMobile(false)}>
                                <Icon />
                                <span className='group-data-[collapsible=icon]:hidden'>{text}</span>
                            </Link>
                        </SidebarMenuButton>
                    </SidebarMenuItem>
                )
            })}
        </SidebarMenu>
    )
}

const ActiveShellNavMenuItems: FC<ShellNavMenuProps> = ({ items }) => {
    const pathname = usePathname()

    return <ShellNavMenuItems items={items} pathname={pathname} />
}

export const ShellNavMenu: FC<ShellNavMenuProps> = ({ items }) => (
    <Suspense fallback={<ShellNavMenuItems items={items} pathname={null} />}>
        <ActiveShellNavMenuItems items={items} />
    </Suspense>
)

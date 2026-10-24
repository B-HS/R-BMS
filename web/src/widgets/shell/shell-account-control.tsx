'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { CircleUserRound, FolderHeart, LogIn, LogOut, Settings, ShieldCheck, UserRound } from 'lucide-react'
import { useTranslations } from 'next-intl'
import { toast } from 'sonner'
import { useSessionLogout, useSessionUser } from '@entities/auth.query'
import { buildLoginHref, DEFAULT_NEXT_PATH, LOGIN_PATH, PROTECTED_PATH_PREFIXES } from '@shared/constants/auth-route'
import { FOOTER_MENU_ITEM_CLASS_NAME, FOOTER_TEXT_BUTTON_CLASS_NAME } from '@shared/constants/shell-footer'
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@shared/ui/dropdown-menu'
import { SidebarMenuButton, SidebarMenuItem, useSidebar } from '@shared/ui/sidebar'
import { Skeleton } from '@shared/ui/skeleton'

const AUTH_PATHS = [LOGIN_PATH, '/signup']
const ADMIN_ROLE = 'admin'

export const ShellAccountControl: FC = () => {
    const router = useRouter()
    const pathname = usePathname()
    const t = useTranslations('common.shell')
    const { isMobile, setOpenMobile } = useSidebar()
    const { user, isPending } = useSessionUser()
    const logout = useSessionLogout()

    const closeMobileSidebar = () => isMobile && setOpenMobile(false)

    const handleSignOut = () =>
        logout.mutate(undefined, {
            onSuccess: () => {
                toast.success(t('signedOut'))
                if (PROTECTED_PATH_PREFIXES.some((prefix) => pathname.startsWith(prefix))) window.location.assign(DEFAULT_NEXT_PATH)
                else router.refresh()
            },
        })

    return (
        <SidebarMenuItem className={FOOTER_MENU_ITEM_CLASS_NAME}>
            {isPending && (
                <div
                    role='status'
                    aria-label={t('sessionChecking')}
                    aria-busy='true'
                    className='flex h-full w-full items-center justify-center gap-2 px-3 group-data-[collapsible=icon]:px-1'>
                    <Skeleton className='size-4' />
                    <Skeleton className='h-3 min-w-0 flex-1 group-data-[collapsible=icon]:hidden' />
                </div>
            )}
            {!isPending && !user && (
                <SidebarMenuButton asChild className={FOOTER_TEXT_BUTTON_CLASS_NAME} tooltip={t('signIn')}>
                    <Link
                        href={AUTH_PATHS.includes(pathname) ? LOGIN_PATH : buildLoginHref(pathname)}
                        aria-label={t('signIn')}
                        onClick={closeMobileSidebar}>
                        <LogIn />
                        <span className='min-w-0 flex-1 truncate text-left group-data-[collapsible=icon]:hidden'>{t('signIn')}</span>
                    </Link>
                </SidebarMenuButton>
            )}
            {!isPending && user && (
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <SidebarMenuButton className={FOOTER_TEXT_BUTTON_CLASS_NAME} aria-label={t('accountMenu')} tooltip={t('accountMenu')}>
                            <UserRound />
                            <span className='min-w-0 flex-1 truncate text-left group-data-[collapsible=icon]:hidden'>{user.name}</span>
                        </SidebarMenuButton>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align='end' side='top' className='w-60'>
                        <DropdownMenuLabel className='grid gap-0.5'>
                            <span className='truncate'>{user.name}</span>
                            <span className='text-muted-foreground truncate font-mono text-xs font-normal'>{user.login_id}</span>
                        </DropdownMenuLabel>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem asChild>
                            <Link href={`/players/${encodeURIComponent(user.login_id)}`} onClick={closeMobileSidebar}>
                                <CircleUserRound />
                                {t('myPage')}
                            </Link>
                        </DropdownMenuItem>
                        <DropdownMenuItem asChild>
                            <Link href='/folders' onClick={closeMobileSidebar}>
                                <FolderHeart />
                                {t('myFolders')}
                            </Link>
                        </DropdownMenuItem>
                        <DropdownMenuItem asChild>
                            <Link href='/settings' onClick={closeMobileSidebar}>
                                <Settings />
                                {t('settings')}
                            </Link>
                        </DropdownMenuItem>
                        {user.role === ADMIN_ROLE && (
                            <DropdownMenuItem asChild>
                                <Link href='/admin/builds' onClick={closeMobileSidebar}>
                                    <ShieldCheck />
                                    {t('buildAdmin')}
                                </Link>
                            </DropdownMenuItem>
                        )}
                        <DropdownMenuSeparator />
                        <DropdownMenuItem disabled={logout.isPending} onSelect={handleSignOut}>
                            <LogOut />
                            {t('signOut')}
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
            )}
        </SidebarMenuItem>
    )
}

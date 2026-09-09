'use client'

import type { FC } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { NavList } from '@features/shell/nav-list'
import { ThemeToggle } from '@widgets/shell/theme-toggle'

export const NavRail: FC = () => {
    const pathname = usePathname()

    return (
        <nav aria-label='주 메뉴' className='bg-sidebar text-sidebar-foreground hidden w-64 shrink-0 flex-col md:flex'>
            <div className='flex h-12 shrink-0 items-center px-3'>
                <Link href='/' className='text-foreground font-mono text-sm tracking-tight'>
                    rbms IR
                </Link>
            </div>
            <NavList pathname={pathname} />
            <div className='flex h-12 shrink-0 items-center justify-between px-3'>
                <Link href='/login' className='text-muted-foreground text-xs'>
                    로그인
                </Link>
                <ThemeToggle />
            </div>
        </nav>
    )
}

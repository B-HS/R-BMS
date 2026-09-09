'use client'

import { useState, type FC } from 'react'
import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { Menu } from 'lucide-react'
import { NavList } from '@features/shell/nav-list'
import { cn } from '@shared/lib/cn'
import { Button } from '@shared/ui/button'
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetTrigger } from '@shared/ui/sheet'
import { ThemeToggle } from '@widgets/shell/theme-toggle'

const MOBILE_SHEET_WIDTH = 'w-72'

export const MobileNav: FC = () => {
    const pathname = usePathname()
    const [isOpen, setIsOpen] = useState(false)

    return (
        <header className='bg-sidebar text-sidebar-foreground flex h-12 shrink-0 items-center justify-between px-3 md:hidden'>
            <Sheet open={isOpen} onOpenChange={setIsOpen}>
                <SheetTrigger asChild>
                    <Button variant='ghost' size='icon' aria-label='메뉴 열기'>
                        <Menu className='size-4' aria-hidden />
                    </Button>
                </SheetTrigger>
                <SheetContent side='left' className={cn('bg-sidebar text-sidebar-foreground gap-0 p-0', MOBILE_SHEET_WIDTH)}>
                    <SheetHeader className='h-12 justify-center px-3'>
                        <SheetTitle className='text-foreground font-mono text-sm tracking-tight'>rbms IR</SheetTitle>
                    </SheetHeader>
                    <nav aria-label='주 메뉴'>
                        <NavList pathname={pathname} onNavigate={() => setIsOpen(false)} />
                    </nav>
                    <Link href='/login' onClick={() => setIsOpen(false)} className='text-muted-foreground flex h-12 items-center px-3 text-xs'>
                        로그인
                    </Link>
                </SheetContent>
            </Sheet>
            <Link href='/' className='text-foreground font-mono text-sm tracking-tight'>
                rbms IR
            </Link>
            <ThemeToggle />
        </header>
    )
}

import type { FC } from 'react'
import Link from 'next/link'
import { BookOpen, Gauge, Layers, Search, Settings, Trophy } from 'lucide-react'
import { NAV_ITEMS, type NavItem } from '@shared/constants/navigation'
import { cn } from '@shared/lib/cn'

const ICONS = {
    gauge: Gauge,
    search: Search,
    trophy: Trophy,
    layers: Layers,
    settings: Settings,
    book: BookOpen,
}

const isActive = (pathname: string, href: NavItem['href']) => (href === '/' ? pathname === '/' : pathname.startsWith(href))

type NavListProps = {
    pathname: string
    onNavigate?: () => void
}

export const NavList: FC<NavListProps> = ({ pathname, onNavigate }) => (
    <ul className='flex min-h-0 flex-1 flex-col overflow-y-auto'>
        {NAV_ITEMS.map((item) => {
            const Icon = ICONS[item.icon]
            return (
                <li key={item.href}>
                    <Link
                        href={item.href}
                        onClick={onNavigate}
                        aria-current={isActive(pathname, item.href) ? 'page' : undefined}
                        className={cn(
                            'flex h-9 items-center gap-2 px-3 text-sm transition-colors',
                            isActive(pathname, item.href) ? 'bg-sidebar-accent text-sidebar-accent-foreground' : 'hover:bg-sidebar-accent/60',
                        )}>
                        <Icon className='size-4 shrink-0' aria-hidden />
                        <span className='truncate'>{item.label}</span>
                    </Link>
                </li>
            )
        })}
    </ul>
)

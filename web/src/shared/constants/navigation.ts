import { BookOpen, FolderHeart, House, Layers, Search, Trophy, type LucideIcon } from 'lucide-react'

export type NavItem = {
    href: string
    label: string
    icon: LucideIcon
}

export const NAV_ITEMS: readonly NavItem[] = [
    { href: '/', label: '홈', icon: House },
    { href: '/search', label: '차트 검색', icon: Search },
    { href: '/leaderboards', label: '순위', icon: Trophy },
    { href: '/guide', label: '가이드', icon: BookOpen },
    { href: '/folders', label: '내 폴더', icon: FolderHeart },
    { href: '/tables', label: '난이도표', icon: Layers },
]

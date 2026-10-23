import { BookOpen, FolderHeart, House, Layers, ListOrdered, Search, Sparkles, Trophy, type LucideIcon } from 'lucide-react'

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
    { href: '/courses', label: '코스', icon: ListOrdered },
    { href: '/ai', label: 'AI', icon: Sparkles },
]

export const SETTINGS_TAB_PARAM = 'tab'
export const AI_SETTINGS_TAB = 'ai'
export const AI_SETTINGS_HREF = `/settings?${SETTINGS_TAB_PARAM}=${AI_SETTINGS_TAB}`
export const DAN_SETTINGS_TAB = 'dans'
export const DAN_SETTINGS_HREF = `/settings?${SETTINGS_TAB_PARAM}=${DAN_SETTINGS_TAB}`

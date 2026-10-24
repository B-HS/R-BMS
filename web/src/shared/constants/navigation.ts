import { BookOpen, FolderHeart, House, Layers, ListOrdered, Search, Sparkles, Trophy, type LucideIcon } from 'lucide-react'

export type NavItem = {
    href: string
    labelKey: 'home' | 'search' | 'leaderboards' | 'guide' | 'folders' | 'tables' | 'courses' | 'ai'
    icon: LucideIcon
}

export const NAV_ITEMS: readonly NavItem[] = [
    { href: '/', labelKey: 'home', icon: House },
    { href: '/search', labelKey: 'search', icon: Search },
    { href: '/leaderboards', labelKey: 'leaderboards', icon: Trophy },
    { href: '/guide', labelKey: 'guide', icon: BookOpen },
    { href: '/folders', labelKey: 'folders', icon: FolderHeart },
    { href: '/tables', labelKey: 'tables', icon: Layers },
    { href: '/courses', labelKey: 'courses', icon: ListOrdered },
    { href: '/ai', labelKey: 'ai', icon: Sparkles },
]

export const SETTINGS_TAB_PARAM = 'tab'
export const AI_SETTINGS_TAB = 'ai'
export const AI_SETTINGS_HREF = `/settings?${SETTINGS_TAB_PARAM}=${AI_SETTINGS_TAB}`
export const DAN_SETTINGS_TAB = 'dans'
export const DAN_SETTINGS_HREF = `/settings?${SETTINGS_TAB_PARAM}=${DAN_SETTINGS_TAB}`

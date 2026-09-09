export const NAV_ITEMS = [
    { href: '/', label: '대시보드', icon: 'gauge' },
    { href: '/search', label: '차트 검색', icon: 'search' },
    { href: '/leaderboards', label: '리더보드', icon: 'trophy' },
    { href: '/tables', label: '난이도표', icon: 'layers' },
    { href: '/settings', label: '설정', icon: 'settings' },
    { href: '/guide', label: '연동 가이드', icon: 'book' },
] as const

export type NavItem = (typeof NAV_ITEMS)[number]

import type { Locale } from '@shared/i18n/locale'

export const SITE_NAME = 'R-BMS'
export const SITE_URL = 'https://bms.hyuns.uk'
export const OPEN_GRAPH_LOCALE = { ko: 'ko_KR', en: 'en_US', ja: 'ja_JP' } as const satisfies Record<Locale, string>
export const SITEMAP_STATIC_PATHS = ['/', '/search', '/leaderboards', '/tables', '/courses', '/guide', '/login', '/signup']
export const ROBOTS_DISALLOW_PATHS = ['/api/', '/settings', '/admin', '/folders']

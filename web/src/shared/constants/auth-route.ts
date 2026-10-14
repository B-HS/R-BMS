export const LOGIN_PATH = '/login'
export const DEFAULT_NEXT_PATH = '/'
export const NEXT_PARAM = 'next'

export const PROTECTED_PATH_PREFIXES = ['/settings', '/admin', '/folders']

export const isSameOriginPath = (path: string) => path.startsWith('/') && !path.startsWith('//') && !path.startsWith('/\\')

export const buildLoginHref = (next: string) => `${LOGIN_PATH}?${NEXT_PARAM}=${encodeURIComponent(next)}`

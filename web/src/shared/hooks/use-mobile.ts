'use client'

import { useSyncExternalStore } from 'react'

const MOBILE_BREAKPOINT_PX = 768
const MOBILE_MEDIA_QUERY = `(max-width: ${MOBILE_BREAKPOINT_PX - 1}px)`

const subscribe: Parameters<typeof useSyncExternalStore>[0] = (onChange) => {
    const mediaQuery = window.matchMedia(MOBILE_MEDIA_QUERY)
    mediaQuery.addEventListener('change', onChange)

    return () => mediaQuery.removeEventListener('change', onChange)
}

const getSnapshot = () => window.matchMedia(MOBILE_MEDIA_QUERY).matches
const getServerSnapshot = () => false

export const useIsMobile = () => useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot)

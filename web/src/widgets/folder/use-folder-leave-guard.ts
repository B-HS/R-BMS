'use client'

import { useEffect, useEffectEvent } from 'react'

const PRIMARY_BUTTON = 0

const readSameOriginTarget = (event: MouseEvent) => {
    if (event.defaultPrevented || event.button !== PRIMARY_BUTTON || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return null
    const anchor = event.target instanceof Element ? event.target.closest('a[href]') : null
    if (!(anchor instanceof HTMLAnchorElement) || anchor.hasAttribute('download')) return null
    if (anchor.target && anchor.target !== '_self') return null
    const url = new URL(anchor.href, window.location.href)
    if (url.origin !== window.location.origin) return null
    if (url.pathname === window.location.pathname && url.search === window.location.search) return null
    return `${url.pathname}${url.search}${url.hash}`
}

/** Stops same-tab link navigation inside the app while `isEnabled`, handing the blocked path to `onBlocked` so the page can ask first. */
export const useFolderLeaveGuard = (isEnabled: boolean, onBlocked: (href: string) => void) => {
    const handleBlocked = useEffectEvent(onBlocked)

    useEffect(() => {
        if (!isEnabled) return

        const handleClick = (event: MouseEvent) => {
            const href = readSameOriginTarget(event)
            if (href === null) return
            event.preventDefault()
            event.stopPropagation()
            handleBlocked(href)
        }

        document.addEventListener('click', handleClick, true)

        return () => document.removeEventListener('click', handleClick, true)
    }, [isEnabled])
}

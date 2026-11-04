'use client'

import { useEffect, useState } from 'react'
import { DASHBOARD_IN_VIEW_ROOT_MARGIN } from '@shared/constants/dashboard'

const isObserverMissing = () => typeof IntersectionObserver === 'undefined'

/**
 * Tells a component when its element has first come near the screen, so a panel can start its own requests only once it is about to be
 * seen. The element is handed over through the returned `ref` callback. Once seen it stays seen, and a browser without
 * `IntersectionObserver` counts every element as seen at once.
 */
export const useInView = () => {
    const [element, setElement] = useState<Element | null>(null)
    const [hasEntered, setHasEntered] = useState(false)

    useEffect(() => {
        if (!element || hasEntered || isObserverMissing()) return
        const observer = new IntersectionObserver((entries) => setHasEntered((seen) => seen || entries.some((entry) => entry.isIntersecting)), {
            rootMargin: DASHBOARD_IN_VIEW_ROOT_MARGIN,
        })
        observer.observe(element)
        return () => observer.disconnect()
    }, [element, hasEntered])

    return { ref: setElement, isInView: hasEntered || isObserverMissing() }
}

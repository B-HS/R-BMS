'use client'

import { type RefObject, useEffect, useState } from 'react'
import { SCROLL_INDICATOR_HIDE_DELAY_MS } from '@shared/constants/ui'
import { calculateScrollThumb, IDLE_SCROLL_THUMB } from '@shared/lib/scroll-thumb'

const readElementMetrics = (element: HTMLElement) => ({
    scrollTop: element.scrollTop,
    scrollHeight: element.scrollHeight,
    clientHeight: element.clientHeight,
})

const readWindowMetrics = () => ({
    scrollTop: window.scrollY,
    scrollHeight: document.documentElement.scrollHeight,
    clientHeight: window.innerHeight,
})

export const useScrollThumb = (targetRef?: RefObject<HTMLElement | null>) => {
    const [thumb, setThumb] = useState(IDLE_SCROLL_THUMB)
    const [isVisible, setIsVisible] = useState(false)

    useEffect(() => {
        const element = targetRef ? targetRef.current : null
        if (targetRef && !element) return

        const scrollTarget = element ?? window
        let frameId = 0
        let hideTimerId = 0
        let shouldReveal = false

        const flush = () => {
            frameId = 0
            const nextThumb = calculateScrollThumb(element ? readElementMetrics(element) : readWindowMetrics())
            setThumb((previous) =>
                previous.isScrollable === nextThumb.isScrollable &&
                previous.heightPercent === nextThumb.heightPercent &&
                previous.topPercent === nextThumb.topPercent
                    ? previous
                    : nextThumb,
            )
            if (!shouldReveal || !nextThumb.isScrollable) return
            shouldReveal = false
            setIsVisible(true)
            window.clearTimeout(hideTimerId)
            hideTimerId = window.setTimeout(() => setIsVisible(false), SCROLL_INDICATOR_HIDE_DELAY_MS)
        }

        const schedule = (reveal: boolean) => {
            shouldReveal = shouldReveal || reveal
            if (frameId === 0) frameId = window.requestAnimationFrame(flush)
        }

        const handleScroll = () => schedule(true)
        const handleLayoutChange = () => schedule(false)

        const resizeObserver = new ResizeObserver(handleLayoutChange)
        const observeLayout = () => {
            resizeObserver.disconnect()
            if (!element) {
                resizeObserver.observe(document.documentElement)
                resizeObserver.observe(document.body)
                return
            }
            resizeObserver.observe(element)
            for (const child of element.children) resizeObserver.observe(child)
        }
        const mutationObserver = new MutationObserver(() => {
            observeLayout()
            handleLayoutChange()
        })

        observeLayout()
        if (element) mutationObserver.observe(element, { childList: true })
        scrollTarget.addEventListener('scroll', handleScroll, { passive: true })
        if (!element) window.addEventListener('resize', handleLayoutChange)
        schedule(false)

        return () => {
            scrollTarget.removeEventListener('scroll', handleScroll)
            if (!element) window.removeEventListener('resize', handleLayoutChange)
            resizeObserver.disconnect()
            mutationObserver.disconnect()
            window.cancelAnimationFrame(frameId)
            window.clearTimeout(hideTimerId)
        }
    }, [targetRef])

    return { ...thumb, isVisible }
}

import { SCROLL_INDICATOR_MIN_OVERFLOW_PX, SCROLL_INDICATOR_MIN_THUMB_PERCENT } from '@shared/constants/ui'

const FULL_PERCENT = 100

export type ScrollMetrics = { scrollTop: number; scrollHeight: number; clientHeight: number }

export const IDLE_SCROLL_THUMB = { heightPercent: 0, topPercent: 0, isScrollable: false }

export const calculateScrollThumb = ({ scrollTop, scrollHeight, clientHeight }: ScrollMetrics) => {
    const overflow = scrollHeight - clientHeight
    if (!Number.isFinite(overflow) || scrollHeight <= 0 || clientHeight <= 0 || overflow < SCROLL_INDICATOR_MIN_OVERFLOW_PX) return IDLE_SCROLL_THUMB

    const heightPercent = Math.max((clientHeight / scrollHeight) * FULL_PERCENT, SCROLL_INDICATOR_MIN_THUMB_PERCENT)
    const travelPercent = FULL_PERCENT - heightPercent
    const progress = Number.isFinite(scrollTop) ? Math.min(Math.max(scrollTop / overflow, 0), 1) : 0

    return { heightPercent, topPercent: progress * travelPercent, isScrollable: true }
}
